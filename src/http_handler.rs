use std::time::{Duration, SystemTime};

use lambda_http::{Body, Error, Request, RequestExt, Response};
use openssl::{base64, error::ErrorStack, hash::MessageDigest, nid::Nid, pkey::{PKey, Public}, rsa::Rsa, sign::Verifier, x509::X509};

use crate::{info_upload::info_upload, media_upload::media_upload_url, post_download::{get_info, get_media_url}, recommendations::recommend_posts};

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let Some(Ok(username)) = event.headers().get("X-Username").map(|it| it.to_str()) else {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "text/plain")
            .body("400 - Missing Header".into())
            .map_err(Box::new)?);
    };
    let Some(Ok(cert_str)) = event.headers().get("X-Auth-Cert").map(|it| it.to_str()) else {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "text/plain")
            .body("400 - Missing Header".into())
            .map_err(Box::new)?);
    };
    let Some(Ok(sig_str)) = event.headers().get("X-Auth-Signature").map(|it| it.to_str()) else {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "text/plain")
            .body("400 - Missing Header".into())
            .map_err(Box::new)?);
    };
    let Some(Ok(nonce)) = event.headers().get("X-Nonce").map(|it| it.to_str()) else {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "text/plain")
            .body("400 - Missing Header".into())
            .map_err(Box::new)?);
    };
    let cert = X509::from_der(&base64::decode_block(cert_str).unwrap()).unwrap();
    let signature = base64::decode_block(sig_str).unwrap();
    let cert_pub_key = cert.public_key().unwrap();
    let mut verifier = Verifier::new(MessageDigest::sha256(), &cert_pub_key).unwrap();
    let reqw_client = reqwest::Client::new();
    let pub_key_req = reqw_client.get("https://social-media-account-provisioning-public-key.s3.us-west-2.amazonaws.com/server_public_key.der").build().unwrap();
    let pub_key_bytes = base64::decode_block(&String::from_utf8(reqw_client.execute(pub_key_req).await.unwrap().bytes().await.unwrap().to_vec()).unwrap()).unwrap();
    let pub_key = PKey::from_rsa(Rsa::public_key_from_der(&pub_key_bytes).unwrap()).unwrap();
    println!("inputs good");


    if let Ok(true) = cert.verify(&pub_key) {} else {
        return Ok(Response::builder()
            .status(401)
            .header("content-type", "text/plain")
            .body("401 - Unauthorized".into())
            .map_err(Box::new)?);
    }
    println!("cert valid");
    if let Ok(true) = verify_cert(&cert, username, &pub_key) {} else {
        return Ok(Response::builder()
            .status(401)
            .header("content-type", "text/plain")
            .body("401 - Unauthorized".into())
            .map_err(Box::new)?);
    }
    println!("username matches cert");
    println!("nonce: {}", nonce);
    if let Ok(timestamp_millis) = nonce.parse::<u64>() {
        let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        let timestamp = Duration::from_millis(timestamp_millis);
        if timestamp > now {
            if timestamp - now > Duration::from_secs(60) {
                return Ok(Response::builder()
                .status(401)
                .header("content-type", "text/plain")
                .body("401 - Unauthorized".into())
                .map_err(Box::new)?);
            }
        } else {
            if now - timestamp > Duration::from_secs(60) {
                return Ok(Response::builder()
                    .status(401)
                    .header("content-type", "text/plain")
                    .body("401 - Unauthorized".into())
                    .map_err(Box::new)?);
            }
        }
    } else {
        println!("nonce not valid number");
        return Ok(Response::builder()
            .status(401)
            .header("content-type", "text/plain")
            .body("401 - Unauthorized".into())
            .map_err(Box::new)?);
    }
    println!("timestamp valid");
    let mut payload = vec![];
    payload.append(&mut username.as_bytes().to_vec());
    payload.append(&mut nonce.as_bytes().to_vec());
    verifier.update(&payload).unwrap();
    if let Ok(true) = verifier.verify(&signature) {} else {
        return Ok(Response::builder()
            .status(401)
            .header("content-type", "text/plain")
            .body("401 - Unauthorized".into())
            .map_err(Box::new)?);
    }
    println!("signature valid");
    println!("{}", event.raw_http_path());
    if event.raw_http_path() == "/post-info" {
        return info_upload(event).await;
    }
    if event.raw_http_path() == "/post-media" {
        return media_upload_url(event).await;
    }
    if event.raw_http_path() == "/get-info" {
        return get_info(event).await;
    }
    if event.raw_http_path() == "/get-media" {
        return get_media_url(event).await;
    }
    if event.raw_http_path() == "/recommendations" {
        return recommend_posts(event).await;
    }

    let resp = Response::builder()
        .status(404)
        .header("content-type", "text/plain")
        .body("404 - Not Found".into())
        .map_err(Box::new)?;
    Ok(resp)
}

fn verify_cert(cert: &X509, username: &str, server_pub_key: &PKey<Public>) -> Result<bool, ErrorStack> {
    if !cert.verify(server_pub_key)? {
        return Ok(false);
    }
    if let Some(field) = cert.subject_name().entries_by_nid(Nid::ACCOUNT).next() {
        if field.data().as_utf8()?.to_string() == username {
            return Ok(true);
        }
    }
    Ok(false)
}