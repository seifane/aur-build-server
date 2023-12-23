use std::collections::HashMap;
use std::convert::Infallible;
use bytes::BufMut;
use futures_util::TryStreamExt;
use warp::multipart::FormData;

#[derive(Debug)]
pub enum MultipartField {
    Text(String),
    File(String, Vec<u8>),
}

pub async fn parse_multipart(form: FormData) -> Result<HashMap<String, Vec<MultipartField>>, Infallible>
{
    let mut fields = HashMap::new();

    let mut parsed_fields: Vec<_> = form
        .and_then(|mut field| async move {
            let mut bytes: Vec<u8> = Vec::new();

            while let Some(content) = field.data().await {
                let content = content.unwrap();
                bytes.put(content);
            }

            let field = match field.filename() {
                None => (
                    field.name().to_string(),
                    MultipartField::Text(String::from_utf8_lossy(&*bytes).to_string())
                ),
                Some(filename) => (
                    field.name().to_string(),
                    MultipartField::File(filename.to_string(), bytes)
                ),
            };

            Ok(field)
        })
        .try_collect()
        .await.unwrap();

    while let Some(f) = parsed_fields.pop() {
        match fields.get_mut(f.0.as_str()) {
            None => {
                fields.insert(f.0, vec![f.1]);
            }
            Some(field) => {
                field.push(f.1);
            }
        }
    }

    Ok(fields)
}