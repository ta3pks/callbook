use axum::extract::Path;
use axum::{middleware::map_response, response::Response, routing::get};
use std::error::Error;

use reqwest::header::CONTENT_TYPE;

#[shuttle_runtime::main]
pub async fn main() -> shuttle_axum::ShuttleAxum {
    let input_html = r#"<form action="/" id="sign_form"><input type="text" autofocus id="callsign"><input type="submit" id="sub" value="Ara"></form>
<script>
window.sign_form.onsubmit = function(e) {
    e.preventDefault();
    var sign = window.callsign.value;
    if (sign) {
        window.location.href = "/" + sign;
    }
};
</script>

                "#;
    Ok(axum::Router::new()
        .route("/", get(|| async { input_html.to_string() }))
        .route(
            "/:callsign",
            get(|Path(callsign): Path<String>| {
                let input_html = input_html.to_string();
                async move {
                    //
                    let callsign = callsign.to_uppercase();
                    let (name, dmr, img) = futures::future::join3(
                        get_name(&callsign),
                        get_dmr_data(&callsign),
                        get_img(&callsign),
                    )
                    .await;
                    let (name, dmr, img) = (
                        name.unwrap_or_default(),
                        dmr.unwrap_or_default().unwrap_or_default(),
                        img.unwrap_or_default().unwrap_or_default(),
                    );
                    format!(
                        r#"
                            <style>
                            table, th, td {{
                                border: 1px solid black;
                                border-collapse: collapse;
                            }}
                            table{{
                            width: 100%;
                            text-align: center;
                            margin:auto auto;
                            }}
                            </style>
                            <table>
                                <thead>
                                    <tr>
                                        <th>çağrı işareti</td>
                                        <th>resim(callbook)</td>
                                        <th>isim(callbook)</td>
                                        <th>isim(dmr)</td>
                                        <th>şehir</td>
                                        <th>ülke</td>
                                        <th>dmrid</td>
                                        <th>bölge</td>
                                    </tr>
                                </thead>
                            <tbody>
                                <tr>
                                    <td>{callsign}</td>
                                    <td><img src="{img}"></td>
                                    <td>{name}</td>
                                    <td>{dmr_fname} {dmr_surname}</td>
                                    <td>{dmr_city}</td>
                                    <td>{dmr_country}</td>
                                    <td>{dmr_id}</td>
                                    <td>{dmr_state}</td>
                                </tr>
                            </tbody>
                            </table>
                            {input_html}
                            "#,
                        callsign = callsign,
                        name = name,
                        dmr_fname = dmr.fname,
                        dmr_surname = dmr.surname,
                        dmr_city = dmr.city,
                        dmr_country = dmr.country,
                        dmr_id = dmr.id,
                        dmr_state = dmr.state,
                        img = img,
                        input_html = input_html
                    )
                }
            }),
        )
        .layer(map_response(|mut resp: Response| async {
            resp.headers_mut()
                .insert("content-type", "text/html; charset=UTF-8".parse().unwrap());
            resp
        }))
        .into())
}
#[derive(Clone, Debug, serde::Deserialize, Default)]
struct DmrData {
    #[serde(default)]
    pub fname: String,
    #[serde(default)]
    pub surname: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub id: usize,
    #[serde(default)]
    pub state: String,
}
async fn get_dmr_data(
    callsign: &str,
) -> Result<Option<DmrData>, Box<dyn Error + 'static + Send + Sync>> {
    #[derive(Debug, serde::Deserialize)]
    struct Response {
        results: Vec<DmrData>,
    }
    let url = format!("https://radioid.net/api/dmr/user/?callsign={callsign}");
    let body = reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .json::<Response>()
        .await?;
    Ok(body.results.first().cloned())
}
async fn get_name(callsign: &str) -> Result<String, Box<dyn Error + 'static + Send + Sync>> {
    let url = format!(
        "http://www.tacallbook.org/cgi-bin/bul1.cgi?ara={}",
        callsign
    );
    let body = reqwest::Client::new()
        .get(&url)
        .header("Referer", "http://www.tacallbook.org/call.shtml")
        .send()
        .await?
        .text()
        .await?;
    let body = if let Some((_, body)) = body.split_once(&callsign.to_uppercase()) {
        body.to_owned()
    } else {
        body
    };
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r#">(.*?[\w ]+)</s"#).unwrap();
        // static ref RE: regex::Regex = regex::Regex::new(r#"strong>([\w ]+)</strong>"#).unwrap();
    };
    let name = RE
        .captures(&body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or_default();
    Ok(name.to_string())
}
async fn get_img(callsign: &str) -> Result<Option<String>, Box<dyn Error + 'static + Send + Sync>> {
    let url = format!("http://www.tacallbook.org/imagestr.php?call={}", callsign);
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Referer", "http://www.tacallbook.org/call.shtml")
        .header("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.7")
        .header("Connection", "keep-alive")
        .header("DNT", "1")
        .header("Sec-GPC", "1")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36")
        .send()
        .await?;
    let headers = resp.headers();
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpg")
        .to_string();
    let body = resp.bytes().await?;
    let body = base64_simd::STANDARD.encode_to_string(&body);
    Ok(Some(format!("data:{};base64,{}", content_type, body)))
}
