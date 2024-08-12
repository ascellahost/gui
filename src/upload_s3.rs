use aws_sdk_s3::{
    config::{
        endpoint::{Endpoint, EndpointFuture, Params, ResolveEndpoint},
        BehaviorVersion, Credentials, Region,
    },
    primitives::ByteStream,
    Client,
};

use crate::ascella_config::AscellaConfig;

#[derive(Debug)]
struct StageResolver(String);
impl ResolveEndpoint for StageResolver {
    fn resolve_endpoint(&self, params: &Params) -> EndpointFuture<'_> {
        let full_url = format!("{}/{}", self.0, params.bucket().unwrap_or(""));
        EndpointFuture::ready(Ok(Endpoint::builder().url(full_url).build()))
    }
}

pub async fn s3_upload(config: &AscellaConfig, name: String, data: Vec<u8>) -> anyhow::Result<()> {
    // let ep = Endpoint::builder()
    //     .url(Uri::from_static(
    //         "https://3dd81dce86015003355892e51e89be8d.r2.cloudflarestorage.com",
    //     ))
    //     .build();

    let identity = Credentials::new(
        config.s3_access_key.clone(),
        config.s3_secret_key.clone(),
        None,
        None,
        "hardcoded-credentials",
    );

    let bucket = config.s3_bucket.clone();

    let config = aws_sdk_s3::Config::builder()
        .region(Region::from_static("auto"))
        .endpoint_resolver(StageResolver(config.s3_url.clone()))
        .behavior_version(BehaviorVersion::v2024_03_28())
        .credentials_provider(identity)
        .build();

    let client = Client::from_conf(config);

    let body = ByteStream::from(data);

    let resp = client
        .put_object()
        .bucket(bucket)
        .key(name)
        .body(body)
        .set_content_type(Some("image/png".to_owned()))
        .send()
        .await?;

    Ok(())
}

// pub async fn main(name: String, data: Vec<u8>) -> anyhow::Result<()> {
//     // Load AWS credentials and S3 bucket info from environment variables
//     let access_key = "103cc42a4d5896537ffc9c37b7db5330".to_owned();
//     let secret_key = "03edb416924eea960fea8142a79832f5c14a52232e942e44c59f374cee6e692f".to_owned();
//     let region = "us-east-1";

//     // The S3 endpoint URL
//     let url = format!("https://3dd81dce86015003355892e51e89be8d.r2.cloudflarestorage.com/images/{name}");

//     // The data you want to upload (as bytes)

//     // Calculate content length and hash of the payload
//     let content_length = data.len();
//     let content_sha256 = Sha256::digest(data.clone());

//     // Set up headers
//     let mut headers = HeaderMap::new();
//     headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
//     headers.insert(CONTENT_LENGTH, HeaderValue::from(content_length as u64));
//     headers.insert(
//         "x-amz-content-sha256",
//         HeaderValue::from_str(&hex::encode(content_sha256))?,
//     );

//     // Prepare the request
//     let client = Client::new();
//     let request = client.put(&url).headers(headers.clone()).body(data.clone()).build()?;

//     // Prepare the string to sign
//     let date = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
//     let date_short = &date[..8];

//     let string_to_sign = format!(
//         "AWS4-HMAC-SHA256\n{}\n{}/{}/{}/aws4_request\n{}",
//         date,
//         date_short,
//         region,
//         "s3",
//         hex::encode(Sha256::digest(request.body().unwrap().as_bytes().unwrap()))
//     );

//     // Calculate the signature
//     let signature = dbg!(generate_signature(
//         &secret_key,
//         date_short,
//         &region,
//         "s3",
//         &string_to_sign
//     ));
//     // Add the Authorization header
//     let authorization = format!(
//         "AWS4-HMAC-SHA256 Credential={}/{}/{}/{}/aws4_request,
// SignedHeaders=content-length;content-type;host;x-amz-content-sha256, Signature={}",         access_key,
//         date_short,
//         region,
//         "s3",
//         signature
//     );
//     headers.insert("Authorization", HeaderValue::from_str(&authorization)?);
//     headers.insert("x-amz-date", HeaderValue::from_str(&date)?);

//     // Send the request
//     let response = dbg!(client.put(&url).headers(headers).body(data.clone()).send().await?);

//     // Handle response
//     // println!("Response: {:?}", response);
//     dbg!(response.text().await?);
//     Ok(())
// }
