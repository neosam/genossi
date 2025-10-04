# OIDC Troubleshooting Guide

This guide helps troubleshoot common OIDC authentication issues in Inventurly.

## Common Errors

### "invalid_client" Error

### Symptoms
```
StandardErrorResponse {
    error: invalid_client,
    error_description: Some("Client authentication failed."),
}
```

### Root Causes and Solutions

#### 1. Missing or Incorrect Client Secret
**Problem**: CLIENT_SECRET environment variable is missing, empty, or incorrect.

**Solution**:
```bash
# Check application logs for:
# "CLIENT_SECRET: NOT_SET" or "CLIENT_SECRET is empty or not set"

# Verify the secret file exists and has content
cat /path/to/your/secret/file
# Should output your actual client secret

# Check systemd service environment (when using NixOS module)
systemctl show inventurly-main --property=Environment
# Should include CLIENT_SECRET

# Check the OIDC environment file was created correctly
sudo cat /var/lib/inventurly-main/oidc.env
# Should contain: CLIENT_SECRET=your_secret_here
```

**Fix**: Set the correct CLIENT_SECRET in your NixOS configuration:
```nix
services.inventurly = {
  oidc = {
    clientSecretFile = "/path/to/secret/file";
  };
};
```

**Note**: The NixOS module uses systemd's `LoadCredential` and `EnvironmentFile` to securely load the client secret. Make sure:
- The secret file exists and is readable by systemd
- The file contains only the client secret (no extra whitespace/newlines)
- The systemd service has proper permissions to read the credential

#### 2. Client Configuration Mismatch
**Problem**: Client ID or redirect URIs don't match OIDC provider configuration.

**Verification Steps**:
1. Check application logs for OIDC configuration:
   ```
   OIDC Configuration:
     APP_URL: http://your-app.com
     ISSUER: https://your-oidc-provider.com
     CLIENT_ID: your-client-id
     CLIENT_SECRET: ***PROVIDED***
   ```

2. Verify in your OIDC provider:
   - Client ID matches exactly
   - Redirect URIs include: `{APP_URL}/auth/authorized`
   - Client type is set to "Confidential" (not "Public")

#### 3. Incorrect App URL
**Problem**: APP_URL doesn't match the actual application URL or redirect URI configuration.

**Common Issues**:
- HTTP vs HTTPS mismatch
- Port number missing or incorrect
- Trailing slash differences

**Solution**: Ensure APP_URL matches exactly how users access your application:
```bash
# Wrong:
APP_URL=http://localhost:3000/

# Correct:
APP_URL=http://localhost:3000
```

#### 4. OIDC Provider Client Type
**Problem**: Client is configured as "Public" instead of "Confidential".

**Solution**: In your OIDC provider settings:
- Set client type to "Confidential"
- Enable "Client Secret" authentication method
- Disable "PKCE" if it's a confidential client

## Environment Variables Checklist

Required environment variables for OIDC:
- `APP_URL`: Base URL of your application (e.g., `https://inventory.example.com`)
- `ISSUER`: OIDC provider issuer URL (e.g., `https://auth.example.com/realms/main`)
- `CLIENT_ID`: Your OIDC client identifier
- `CLIENT_SECRET`: Your OIDC client secret (confidential clients only)

## NixOS Configuration Example

```nix
services.inventurly = {
  enable = true;
  oidc = {
    enable = true;
    issuer = "https://auth.example.com/realms/main";
    clientId = "inventurly-prod";
    clientSecretFile = "/run/secrets/oidc-client-secret";
    appUrl = "https://inventory.example.com";
  };
};

# Create the secret file
sops.secrets.oidc-client-secret = {
  sopsFile = ./secrets.yaml;
  owner = "inventurly";
  mode = "0400";
};
```

## Debug Logging

Enable debug logging to troubleshoot OIDC issues:

1. Check server startup logs for OIDC configuration
2. Look for "OIDC client discovery" success/failure messages
3. Monitor authentication error logs during login attempts

## Testing OIDC Configuration

1. **Test OIDC Discovery Endpoint**:
   ```bash
   curl -s "${ISSUER}/.well-known/openid-configuration" | jq
   ```

2. **Verify Client Configuration**:
   - Login to your OIDC provider admin interface
   - Check client configuration matches your environment variables
   - Verify redirect URIs include your APP_URL + `/auth/authorized`

3. **Test Application Startup**:
   ```bash
   # Build with OIDC feature
   nix-build --arg features '["oidc"]'
   
   # Check logs during startup
   journalctl -u inventurly -f
   ```

## Common Provider-Specific Issues

### Keycloak
- Ensure "Standard Flow" is enabled for the client
- Set "Access Type" to "confidential"
- Add valid redirect URIs: `{APP_URL}/auth/authorized`

### Auth0
- Set "Application Type" to "Regular Web Applications"
- Ensure "Client Secret" is generated and not expired
- Configure "Allowed Callback URLs"

### Azure AD
- Set "Client Secret" in "Certificates & secrets"
- Configure "Redirect URIs" in "Authentication"
- Ensure "Access tokens" and "ID tokens" are enabled

### "FOREIGN KEY constraint failed" Error

**Symptoms:**
```
panicked at inventurly_rest/src/session.rs:43:14:
an `Err` value: DataAccess("DatabaseError(\"error returned from database: (code: 787) FOREIGN KEY constraint failed\")")
```

**Root Cause:**
This error occurs when a user logs in via OIDC for the first time. The session table has a foreign key constraint to the user table, but the user doesn't exist in the database yet.

**Solution:**
This has been fixed in the current version by implementing auto-registration for OIDC users. The system now:
1. Automatically creates a user record when someone logs in via OIDC for the first time
2. Uses the username from the OIDC `preferred_username` claim
3. Creates the user with process = "oidc-auto-register"

**Verification:**
- Check that users are being created in the database after OIDC login
- Verify no more foreign key constraint errors in the logs
- New OIDC users should be able to log in successfully

**Manual Fix (if needed):**
If you encounter this error on an older version, you can manually create users:
```sql
INSERT INTO user (name, update_timestamp, update_process) 
VALUES ('username', NULL, 'manual-oidc-setup');
```

## Still Having Issues?

If you're still experiencing problems:

1. Check the application logs with RUST_LOG=debug for detailed error information
2. Verify your OIDC provider's logs for additional error details
3. Test with a simple OIDC client to isolate configuration issues
4. Ensure network connectivity between your application and OIDC provider

For additional help, create an issue with:
- Complete error logs from the application
- OIDC provider type and version
- Environment variable configuration (redact secrets)
- Steps to reproduce the issue