# OIDC Configuration for NixOS Module

The Inventurly NixOS module now supports OIDC authentication with dedicated configuration options.

## Configuration Examples

### OIDC Authentication (Production)

```nix
services.inventurly.production = {
  enable = true;
  domain = "inventurly.example.com";
  
  oidc = {
    enable = true;
    issuer = "https://accounts.google.com";
    clientId = "your-google-client-id";
    clientSecretFile = /etc/secrets/inventurly-client-secret;
    # appUrl auto-derived as "https://inventurly.example.com"
  };
};
```

### Mock Authentication (Development)

```nix
services.inventurly.development = {
  enable = true;
  domain = "inventurly-dev.example.com";
  port = 3001;
  
  oidc.enable = false; # Uses mock authentication
};
```

### Custom APP_URL

```nix
services.inventurly.custom = {
  enable = true;
  
  oidc = {
    enable = true;
    issuer = "https://login.microsoftonline.com/tenant-id/v2.0";
    clientId = "azure-client-id";
    clientSecretFile = /run/secrets/azure-client-secret;
    appUrl = "https://custom-domain.com:8443"; # Override auto-derivation
  };
};
```

## Configuration Options

### `oidc.enable`
- **Type**: `bool`
- **Default**: `false`
- **Description**: Enable OIDC authentication (disables mock authentication)

### `oidc.issuer`
- **Type**: `string`
- **Required when OIDC enabled**
- **Example**: `"https://accounts.google.com"`
- **Description**: OIDC provider issuer URL

### `oidc.clientId`
- **Type**: `string`
- **Required when OIDC enabled**
- **Description**: OAuth client ID from your OIDC provider

### `oidc.clientSecretFile`
- **Type**: `null or path`
- **Default**: `null`
- **Example**: `/etc/secrets/inventurly-client-secret`
- **Description**: Path to file containing OAuth client secret

### `oidc.appUrl`
- **Type**: `null or string`
- **Default**: Auto-derived from `domain`
- **Example**: `"https://inventurly.example.com"`
- **Description**: Application URL for OIDC callbacks

## Automatic Behavior

### Package Selection
- When `oidc.enable = true`: Uses `features = ["oidc"]`
- When `oidc.enable = false`: Uses `features = ["mock_auth"]`

### URL Derivation
- `APP_URL`: Defaults to `https://${domain}` or falls back to `http://${host}:${port}`
- `BASE_PATH`: Set to `${APP_URL}/`

### Nginx Configuration
- Automatically configures `/authenticate` and `/logout` endpoints
- Only applied when `domain` is set

## Secret Management

The module supports secure secret handling:

1. **File-based**: Use `clientSecretFile` pointing to a file
2. **Systemd credentials**: The secret is loaded via systemd's credential system
3. **Environment variable**: Available as `CLIENT_SECRET` in the service

## Required OIDC Provider Setup

Configure your OIDC provider with:
- **Redirect URI**: `https://your-domain.com/authenticate`
- **Post-logout redirect URI**: `https://your-domain.com/`

## Assertions

The module validates configuration with these checks:
- `oidc.issuer` must be set when OIDC is enabled
- `oidc.clientId` must be set when OIDC is enabled  
- Either `domain` or `oidc.appUrl` must be set when OIDC is enabled

## Migration from Previous Configuration

**Before** (manual extraEnvironment):
```nix
services.inventurly.instance = {
  enable = true;
  extraEnvironment = {
    APP_URL = "https://example.com";
    ISSUER = "https://accounts.google.com";
    CLIENT_ID = "client-id";
    CLIENT_SECRET = "secret";
  };
};
```

**After** (dedicated OIDC options):
```nix
services.inventurly.instance = {
  enable = true;
  domain = "example.com";
  
  oidc = {
    enable = true;
    issuer = "https://accounts.google.com";
    clientId = "client-id";
    clientSecretFile = /etc/secrets/client-secret;
  };
};
```