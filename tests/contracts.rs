use fantasy_lcsc::{http::Http, lcsc::*, models::*};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "search" => include_str!("fixtures/search.json"),
        "detail" => include_str!("fixtures/detail.json"),
        "params" => include_str!("fixtures/params.json"),
        _ => panic!(),
    })
    .unwrap()
}
#[test]
fn domestic_search_contract() {
    let raw = fixture("search");
    let result = search_result(&raw).unwrap();
    assert_eq!(result["totalCount"], 250);
    let product = normalize_product(&result["productRecordList"][0]).unwrap();
    assert_eq!(product["code"], "C7950");
    assert_eq!(product["model"], "LM358DR2G");
    assert_eq!(product["currency"], "CNY");
    assert!(product["prices"].as_array().unwrap().len() > 1);
    assert!(
        product["url"]
            .as_str()
            .unwrap()
            .starts_with("https://item.szlcsc.com/")
    );
}
#[test]
fn complete_detail_and_separate_stock_sources() {
    let p = normalize_detail(&fixture("detail"), None).unwrap();
    assert_eq!(p["code"], "C8734");
    assert_eq!(p["id"], "9243");
    assert_eq!(p["parameters"]["RAM容量"], "20KB");
    assert_eq!(p["parameters"]["工作电压"], "2V~3.6V");
    assert_eq!(p["parameter_list"].as_array().unwrap().len(), 12);
    assert_eq!(p["stock"], p["stock_breakdown"]["total"]);
    assert_ne!(
        p["stock_breakdown"]["record"],
        p["stock_breakdown"]["total"]
    );
    assert!(
        p["datasheets"][0]["url"]
            .as_str()
            .unwrap()
            .starts_with("https://atta.szlcsc.com/")
    );
}
#[test]
fn dynamic_unicode_parameter_contract() {
    let raw = fixture("params");
    let options = parameter_options(api_result(&raw).unwrap()).unwrap();
    let inputs = vec!["阻值=10kΩ".into(), "精度=±1%".into()];
    let pairs = resolve_parameters(&inputs, &options).unwrap();
    assert_eq!(
        encode_parameters(&pairs),
        "10k%CE%A9%40param_10835_n%24%C2%B11%25%40param_10836_s"
    );
    assert!(resolve_parameters(&["阻值=10kohms".into()], &options).is_err());
    assert!(resolve_parameters(&["未知=1".into()], &options).is_err());
    let values = &options
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "阻值")
        .unwrap()["values"];
    assert!(
        values
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"].as_str().unwrap().ends_with("mΩ"))
    );
    assert!(
        values
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"].as_str().unwrap().ends_with("MΩ"))
    );
}
#[test]
fn upstream_errors_never_become_empty_results() {
    assert_eq!(
        search_result(&json!({"code":200,"result":{}}))
            .unwrap_err()
            .code,
        "schema_changed"
    );
    assert!(
        api_result(&json!({"code":403,"msg":"blocked"}))
            .unwrap_err()
            .fallback
    );
    assert!(api_result(&json!({"products":[]})).is_err());
    assert_eq!(parameter_options(&Value::Null).unwrap(), Value::Null);
    assert!(normalize_product(&json!({})).is_err());
}
#[test]
fn identifiers_facets_and_html() {
    assert!(matches!(parse_identifier("c8734").unwrap(),ItemIdentifier::Code(c) if c=="C8734"));
    assert!(
        matches!(parse_identifier("https://item.szlcsc.com/9243.html?from=test").unwrap(),ItemIdentifier::Id(id) if id=="9243")
    );
    for input in [
        "https://item.szlcsc.com.evil.test/9243.html",
        "https://user:pass@item.szlcsc.com/9243.html",
        "file:///C:/secret",
        "C-1",
        "",
    ] {
        assert!(parse_identifier(input).is_err(), "{input}");
    }
    let choices = json!([{"value":74,"label":"ST"},{"value":100,"label":"TI"}]);
    assert_eq!(
        resolve_facet(&["74".into(), "TI".into()], &choices, "品牌").unwrap(),
        "74$100"
    );
    assert!(
        resolve_facet(
            &["ST".into()],
            &json!([{"value":74,"label":"ST"},{"value":75,"label":"ST"}]),
            "品牌"
        )
        .is_err()
    );
    let html = r#"<script type="application/json" id="__NEXT_DATA__">{"props":{"pageProps":{"webData":1}}}</script>"#;
    assert_eq!(extract_next_data(html).unwrap()["webData"], 1);
    assert!(
        extract_next_data("<html>WafCaptcha</html>")
            .unwrap_err()
            .fallback
    );
    assert_eq!(
        plain_text("厚膜<SPAN>电阻</SPAN> &amp; 薄膜"),
        "厚膜电阻 & 薄膜"
    );
}
#[test]
fn invalid_filters_fail_before_network() {
    let mut client = LcscClient::new(Backend::Http, Duration::from_millis(30), None, None).unwrap();
    let o = SearchOptions {
        catalog: vec!["439".into(), "440".into()],
        param: vec!["阻值=10kΩ".into()],
        ..Default::default()
    };
    assert!(
        client
            .prepare("电阻", &o)
            .unwrap_err()
            .message
            .contains("单个类目")
    );
    assert_eq!(
        client
            .prepare("", &SearchOptions::default())
            .unwrap_err()
            .code,
        "invalid_argument"
    );
    let o = SearchOptions {
        min_price: Some(f64::NAN),
        ..Default::default()
    };
    assert_eq!(
        client.prepare("电阻", &o).unwrap_err().code,
        "invalid_argument"
    );
}
const UUID: &str = "76f2cff83c8642c78e11744b05cb5200";
fn component() -> Value {
    json!({"success":true,"result":{"title":"PZ254R-11-03P","dataStr":{"head":{"c_para":{"Supplier Part":"C492411","Manufacturer":"XFCN"}}},"packageDetail":{"title":"CONN-TH","dataStr":json!({"shape":[format!("SVGNODE~{}",json!({"attrs":{"c_etype":"outline3D","uuid":UUID}})),"SVGNODE~bad-json",format!("SVGNODE~{}",json!({"attrs":{"c_etype":"outline3D","uuid":UUID}}))]}).to_string()}}})
}
const STEP:&[u8]=b"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=CARTESIAN_POINT('',(0.,0.,0.));\nENDSEC;\nEND-ISO-10303-21;\n";
#[test]
fn part_metadata_string_data_uuid_validation_and_deduplication() {
    assert_eq!(
        normalize_parts(&["c492411，C492414; C492411\n".into()]).unwrap(),
        ["C492411", "C492414"]
    );
    assert!(normalize_parts(&["C492411 ../evil".into()]).is_err());
    let info = component_info("C492411", &component()).unwrap();
    assert_eq!(info["manufacturer"], "XFCN");
    assert_eq!(info["model_uuids"], json!([UUID]));
    assert!(info["currency"].is_null());
    assert_eq!(safe_title("../../TYPE-C 16PIN(073)"), "TYPE-C_16PIN_073");
    assert!(component_info("C123", &component()).is_err());
}
#[test]
fn step_validation_rejects_error_pages_and_truncation() {
    validate_step(STEP).unwrap();
    for data in [
        b"<html>404</html>".as_slice(),
        &STEP[..STEP.len() - 20],
        b"ISO-10303-21; HEADER; DATA; END-ISO-10303-21;",
    ] {
        assert!(validate_step(data).is_err());
    }
}
fn server(responses: Vec<(u16, Vec<u8>)>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let mut requests = vec![];
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            let mut line = String::new();
            let mut length = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                head.push_str(&line);
                if line == "\r\n" {
                    break;
                }
                if let Some(n) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = n.trim().parse().unwrap();
                }
            }
            let mut request_body = vec![0; length];
            reader.read_exact(&mut request_body).unwrap();
            requests.push(head);
            write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        }
        requests
    });
    (origin, handle)
}
fn model_client(origin: &str) -> ModelClient {
    ModelClient::with_origins(
        Http::new(Duration::from_secs(5), None).unwrap(),
        format!("{origin}/products"),
        format!("{origin}/step"),
    )
}
#[test]
fn download_atomic_preservation_on_invalid_step() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("C492411_PZ254R-11-03P.step");
    std::fs::write(&target, STEP).unwrap();
    let (origin, handle) = server(vec![
        (200, component().to_string().into_bytes()),
        (200, b"<html>bad</html>".to_vec()),
    ]);
    let error = model_client(&origin)
        .download("C492411", temp.path(), true, &mut |_| true)
        .unwrap_err();
    assert_eq!(error.code, "invalid_step");
    assert_eq!(std::fs::read(&target).unwrap(), STEP);
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    assert_eq!(handle.join().unwrap().len(), 2);
}
#[test]
fn download_retries_and_skips_complete_files() {
    let temp = tempfile::tempdir().unwrap();
    let (origin, handle) = server(vec![
        (200, component().to_string().into_bytes()),
        (503, vec![]),
        (200, STEP.to_vec()),
    ]);
    let files = model_client(&origin)
        .download("C492411", temp.path(), false, &mut |_| true)
        .unwrap();
    assert!(!files[0].skipped);
    assert_eq!(std::fs::read(&files[0].path).unwrap(), STEP);
    let requests = handle.join().unwrap();
    assert!(requests[0].starts_with("GET /products/C492411/components"));
    assert!(requests[1].starts_with(&format!("GET /step/{UUID}")));
    let (origin, handle) = server(vec![(200, component().to_string().into_bytes())]);
    let files = model_client(&origin)
        .download("C492411", temp.path(), false, &mut |_| true)
        .unwrap();
    assert!(files[0].skipped);
    assert_eq!(handle.join().unwrap().len(), 1);
}
#[test]
fn cancellation_does_not_publish_a_file() {
    let temp = tempfile::tempdir().unwrap();
    let (origin, handle) = server(vec![
        (200, component().to_string().into_bytes()),
        (200, STEP.to_vec()),
    ]);
    let error=model_client(&origin).download("C492411",temp.path(),true,&mut |event| !matches!(event,DownloadEvent::Progress {count:1,received,..} if received>0)).unwrap_err();
    assert_eq!(error.code, "cancelled");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    handle.join().unwrap();
}
#[test]
fn cli_invalid_arguments_are_structured_and_exit_two() {
    for args in [
        vec!["search", "LM358", "--page", "0", "--json"],
        vec!["download", "bad", "--json"],
        vec!["search", "", "--json"],
        vec!["detail", "https://evil.test/1.html", "--json"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_fantasy-lcsc"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_argument");
    }
}
#[test]
fn noclobber_write_protects_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("existing.step");
    std::fs::write(&path, STEP).unwrap();
    assert!(fantasy_lcsc::atomic_write(&path, b"replacement", false).is_err());
    assert_eq!(std::fs::read(path).unwrap(), STEP);
}

#[test]
fn update_requires_installation_and_rejects_conflicting_modes() {
    for args in [
        vec!["update", "--check", "--local", "--json"],
        vec!["update", "--raw"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_fantasy-lcsc"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let value: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["error"]["code"], "invalid_argument");
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_fantasy-lcsc"))
            .args(["update", "--json"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let value: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["error"]["code"], "not_installed");
    }
}
