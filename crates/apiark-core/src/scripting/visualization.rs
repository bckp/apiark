use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Serialize, Deserialize};
use std::io::Cursor;

pub const MAX_OUTPUTS: usize = 4;
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 256 * 1024;
pub const MAX_DIMENSION: u32 = 4096;
pub const MAX_PIXELS: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type",rename_all="lowercase")]
pub enum Visualization {
    Image {name:String,data:String,#[serde(rename="mimeType")] mime_type:String,width:u32,height:u32},
    Text {name:String,data:String},
    Json {name:String,data:serde_json::Value},
}

pub fn validate(raw: Option<&serde_json::Value>) -> Result<Vec<Visualization>,String> {
    let Some(raw)=raw else {return Ok(vec![])};
    let entries=raw.as_array().ok_or("Visualization outputs must be an array")?;
    if entries.len()>MAX_OUTPUTS {return Err("Maximum 4 visualization outputs per request".into());}
    let mut bytes_used=0;let mut result=Vec::new();
    for item in entries {
        let name=item.get("name").and_then(|v|v.as_str()).unwrap_or("Visualization");
        if name.chars().count()>100 {return Err("Visualization name exceeds 100 characters".into());}
        let kind=item.get("type").and_then(|v|v.as_str()).ok_or("Missing visualization type")?;
        let data=item.get("data").ok_or("Missing visualization data")?;
        match kind {
            "image"=>{
                let mime=item.get("mimeType").and_then(|v|v.as_str()).ok_or("Image requires mimeType")?;
                let format=match mime {"image/png"=>image::ImageFormat::Png,"image/jpeg"=>image::ImageFormat::Jpeg,_=>return Err("Only PNG and JPEG are supported; HTML, SVG and remote URLs are disabled".into())};
                let encoded=data.as_str().ok_or("Image data must be a Base64 string")?;
                if encoded.len()>MAX_IMAGE_BYTES.div_ceil(3)*4 {return Err("Image exceeds 4 MiB decoded limit".into());}
                let bytes=STANDARD.decode(encoded).map_err(|_|"Image requires raw standard Base64, without data: or remote URLs")?;
                bytes_used+=bytes.len();
                if bytes_used>MAX_IMAGE_BYTES {return Err("Combined visualization data exceeds 4 MiB".into());}
                if image::guess_format(&bytes).map_err(|e|e.to_string())? !=format {return Err("Image signature does not match mimeType".into());}
                let reader=image::ImageReader::with_format(Cursor::new(&bytes),format);
                let (width,height)=reader.into_dimensions().map_err(|e|e.to_string())?;
                if width==0 || height==0 || width>MAX_DIMENSION || height>MAX_DIMENSION || u64::from(width)*u64::from(height)>MAX_PIXELS {return Err("Image exceeds dimension/pixel limits (4096 per axis, 8M pixels)".into());}
                let mut reader=image::ImageReader::with_format(Cursor::new(&bytes),format);
                let mut limits=image::Limits::default();limits.max_image_width=Some(MAX_DIMENSION);limits.max_image_height=Some(MAX_DIMENSION);limits.max_alloc=Some(32*1024*1024);reader.limits(limits);
                let decoded=reader.decode().map_err(|e|e.to_string())?;
                // Canonical static PNG removes animation frames, metadata and trailing payloads.
                let mut png=Cursor::new(Vec::new());decoded.write_to(&mut png,image::ImageFormat::Png).map_err(|e|e.to_string())?;
                if png.get_ref().len()>MAX_IMAGE_BYTES {return Err("Canonical image exceeds 4 MiB".into());}
                result.push(Visualization::Image{name:name.into(),data:STANDARD.encode(png.into_inner()),mime_type:"image/png".into(),width,height});
            },
            "text"=>{let text=data.as_str().ok_or("Text data must be a string")?;if text.len()>MAX_TEXT_BYTES{return Err("Text exceeds 256 KiB".into());}bytes_used+=text.len();result.push(Visualization::Text{name:name.into(),data:text.into()});},
            "json"=>{let size=serde_json::to_vec(data).map_err(|e|e.to_string())?.len();if size>MAX_TEXT_BYTES{return Err("JSON exceeds 256 KiB".into());}bytes_used+=size;result.push(Visualization::Json{name:name.into(),data:data.clone()});},
            _=>return Err(format!("Unsupported visualization type: {kind}")),
        }
    }
    if bytes_used>MAX_IMAGE_BYTES {return Err("Combined visualization data exceeds 4 MiB".into());}
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_rasters_and_rejects_active_content_urls_limits_and_bad_signatures() {
        for format in [image::ImageFormat::Png,image::ImageFormat::Jpeg] {
            let mut bytes=Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(2,3).write_to(&mut bytes,format).unwrap();
            let output=validate(Some(&serde_json::json!([{"type":"image","name":"Synthetic","data":STANDARD.encode(bytes.into_inner()),"mimeType":if format==image::ImageFormat::Png {"image/png"}else{"image/jpeg"}}]))).unwrap();
            assert!(matches!(output[0],Visualization::Image{width:2,height:3,..}));
        }
        for entry in [serde_json::json!({"type":"html","data":"<script/>"}),serde_json::json!({"type":"image","mimeType":"image/svg+xml","data":"<svg/>"}),serde_json::json!({"type":"image","mimeType":"image/jpeg","data":"https://example.test/image.jpg"}),serde_json::json!({"type":"image","mimeType":"image/png","data":STANDARD.encode(b"<svg/>")}),serde_json::json!({"type":"text","data":"x".repeat(MAX_TEXT_BYTES+1)})] {
            assert!(validate(Some(&serde_json::json!([entry]))).is_err());
        }
        assert!(validate(Some(&serde_json::json!(vec![serde_json::json!({"type":"text","data":"x"});5]))).is_err());
        let mut bytes=Cursor::new(Vec::new());image::DynamicImage::new_rgb8(4097,1).write_to(&mut bytes,image::ImageFormat::Png).unwrap();
        assert!(validate(Some(&serde_json::json!([{"type":"image","data":STANDARD.encode(bytes.into_inner()),"mimeType":"image/png"}]))).is_err());
    }
}
