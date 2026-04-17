use cmpv2::status::PkiStatus;
use der::asn1::OctetString;
use der::{Any, Decode, Encode};
use sha2::{Digest, Sha256};
use spki::AlgorithmIdentifier;
use std::sync::Arc;
use x509_tsp::{MessageImprint, TimeStampReq, TimeStampResp, TspVersion};

const SHA256_OID: der::asn1::ObjectIdentifier =
    der::asn1::ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");

#[derive(Debug, Clone)]
pub enum TsaError {
    EncodingError(Arc<str>),
    DecodingError(Arc<str>),
    HttpError(Arc<str>),
    TsaRejected(Arc<str>),
}

impl std::fmt::Display for TsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TsaError::EncodingError(e) => write!(f, "Encoding error: {}", e),
            TsaError::DecodingError(e) => write!(f, "Decoding error: {}", e),
            TsaError::HttpError(e) => write!(f, "HTTP error: {}", e),
            TsaError::TsaRejected(e) => write!(f, "TSA rejected: {}", e),
        }
    }
}

pub fn build_timestamp_request(hash: &[u8]) -> Result<Vec<u8>, TsaError> {
    let message_imprint = MessageImprint {
        hash_algorithm: AlgorithmIdentifier::<Any> {
            oid: SHA256_OID,
            parameters: None,
        },
        hashed_message: OctetString::new(hash.to_vec())
            .map_err(|e| TsaError::EncodingError(Arc::from(e.to_string())))?,
    };

    let req = TimeStampReq {
        version: TspVersion::V1,
        message_imprint,
        req_policy: None,
        nonce: None,
        cert_req: true,
        extensions: None,
    };

    req.to_der()
        .map_err(|e| TsaError::EncodingError(Arc::from(e.to_string())))
}

pub fn build_timestamp_request_for_audit_hash(audit_hash: &str) -> Result<Vec<u8>, TsaError> {
    let hash = Sha256::digest(audit_hash.as_bytes());
    build_timestamp_request(&hash)
}

pub fn parse_timestamp_response(response_bytes: &[u8]) -> Result<bool, TsaError> {
    let resp = TimeStampResp::from_der(response_bytes)
        .map_err(|e| TsaError::DecodingError(Arc::from(e.to_string())))?;

    Ok(matches!(
        resp.status.status,
        PkiStatus::Accepted | PkiStatus::GrantedWithMods
    ))
}

pub async fn send_timestamp_request(
    tsa_url: &str,
    request_der: &[u8],
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<u8>, TsaError> {
    let client = reqwest::Client::new();
    let mut req = client
        .post(tsa_url)
        .header("Content-Type", "application/timestamp-query")
        .body(request_der.to_vec());

    if let (Some(user), Some(pass)) = (username, password) {
        req = req.basic_auth(user, Some(pass));
    }

    let response = req
        .send()
        .await
        .map_err(|e| TsaError::HttpError(Arc::from(e.to_string())))?;

    if !response.status().is_success() {
        return Err(TsaError::HttpError(Arc::from(format!(
            "TSA returned HTTP {}",
            response.status()
        ))));
    }

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| TsaError::HttpError(Arc::from(e.to_string())))
}

/// Performs a complete timestamp request: hashes the audit hash, builds the
/// request, sends it to the TSA, parses the response, and returns the raw
/// response bytes (the .tsr token).
pub async fn request_timestamp(
    tsa_url: &str,
    audit_hash: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<u8>, TsaError> {
    let request_der = build_timestamp_request_for_audit_hash(audit_hash)?;
    let response_bytes = send_timestamp_request(tsa_url, &request_der, username, password).await?;

    let is_success = parse_timestamp_response(&response_bytes)?;
    if !is_success {
        return Err(TsaError::TsaRejected(Arc::from(
            "TSA returned non-success status",
        )));
    }

    Ok(response_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_timestamp_request_deterministic() {
        let hash = Sha256::digest(b"test data");
        let req1 = build_timestamp_request(&hash).unwrap();
        let req2 = build_timestamp_request(&hash).unwrap();
        assert_eq!(req1, req2);
        assert!(!req1.is_empty());
    }

    #[test]
    fn test_build_timestamp_request_different_hashes() {
        let hash1 = Sha256::digest(b"test data 1");
        let hash2 = Sha256::digest(b"test data 2");
        let req1 = build_timestamp_request(&hash1).unwrap();
        let req2 = build_timestamp_request(&hash2).unwrap();
        assert_ne!(req1, req2);
    }

    #[test]
    fn test_build_timestamp_request_valid_der() {
        let hash = Sha256::digest(b"test");
        let der_bytes = build_timestamp_request(&hash).unwrap();
        let parsed = TimeStampReq::from_der(&der_bytes);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.version, TspVersion::V1);
        assert!(parsed.cert_req);
        assert_eq!(
            parsed.message_imprint.hashed_message.as_bytes(),
            hash.as_slice()
        );
    }

    #[test]
    fn test_build_timestamp_request_for_audit_hash() {
        let req = build_timestamp_request_for_audit_hash("some_audit_hash_value").unwrap();
        assert!(!req.is_empty());
        let parsed = TimeStampReq::from_der(&req).unwrap();
        assert_eq!(parsed.version, TspVersion::V1);
    }

    #[test]
    fn test_parse_invalid_response() {
        let result = parse_timestamp_response(&[0x00, 0x01, 0x02]);
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires network access to freetsa.org
    async fn test_integration_freetsa() {
        let response_bytes = request_timestamp(
            "https://freetsa.org/tsr",
            "test_audit_hash_12345",
            None,
            None,
        )
        .await
        .unwrap();

        assert!(!response_bytes.is_empty());
        let is_success = parse_timestamp_response(&response_bytes).unwrap();
        assert!(is_success);
    }

    #[test]
    fn test_parse_valid_response() {
        // Known good response from x509-tsp test vectors
        let enc_resp = hex_literal::hex!("3082028430030201003082027B06092A864886F70D010702A082026C30820268020103310F300D060960864801650304020105003081C9060B2A864886F70D0109100104A081B90481B63081B302010106042A0304013031300D060960864801650304020105000420BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD020104180F32303233303630373131323632365A300A020101800201F48101640101FF0208314CFCE4E0651827A048A4463044310B30090603550406130255533113301106035504080C0A536F6D652D5374617465310D300B060355040A0C04546573743111300F06035504030C0854657374205453413182018430820180020101305C3044310B30090603550406130255533113301106035504080C0A536F6D652D5374617465310D300B060355040A0C04546573743111300F06035504030C08546573742054534102146A0DCC59137C11D1C2B092042B4BC51C0D634D24300D06096086480165030402010500A08198301A06092A864886F70D010903310D060B2A864886F70D0109100104301C06092A864886F70D010905310F170D3233303630373131323632365A302B060B2A864886F70D010910020C311C301A3018301604142F36B1B52456F5AC3A1CA09794AE3D0D64AD38C2302F06092A864886F70D01090431220420BAF4CCF82E9B5B3956EADCC87346B407684F26D82B68D0E7DE0D31EA79AF648C300A06082A8648CE3D0403020467306502305A6E1C175B20A93FAB25D14CC5F5A2836D726D6D4A964B66FFBFFCE46276A96475F1408728B3385DCA37C2BA46BE17E1023100C46B7F08D03409A8ECCFD7637765412C3C5EC050E0D39CF48F0F5015950342CB18D8434FF331BA4463C086297C37D07B");
        let is_success = parse_timestamp_response(&enc_resp).unwrap();
        assert!(is_success);
    }
}
