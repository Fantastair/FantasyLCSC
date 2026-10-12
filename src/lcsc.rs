use crate::{Error, Result, browser::Browser, http::Http};
use clap::{Args, ValueEnum};
use regex::Regex;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::LazyLock, time::Duration};

#[derive(Clone, Copy, Debug, Default, ValueEnum, PartialEq)]
pub enum Backend {
    #[default]
    Auto,
    Http,
    Browser,
}
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum Sort {
    #[default]
    Relevance,
    PriceAsc,
    PriceDesc,
    Sales,
    StockDesc,
    StockAsc,
    GdStockDesc,
    GdStockAsc,
    JsStockDesc,
    JsStockAsc,
}
impl Sort {
    fn number(self) -> u8 {
        match self {
            Self::Relevance => 0,
            Self::PriceAsc => 1,
            Self::PriceDesc => 2,
            Self::Sales => 6,
            Self::StockDesc => 8,
            Self::StockAsc => 9,
            Self::GdStockDesc => 10,
            Self::GdStockAsc => 11,
            Self::JsStockDesc => 12,
            Self::JsStockAsc => 13,
        }
    }
}
#[derive(Clone, Debug, Args)]
pub struct SearchOptions {
    /// 类目 ID 或完整名称，可重复
    #[arg(long)]
    pub catalog: Vec<String>,
    /// 品牌 ID 或完整名称，可重复
    #[arg(long)]
    pub brand: Vec<String>,
    #[arg(long)]
    pub package: Vec<String>,
    /// 参数名=原始值（单位大小写敏感）
    #[arg(long)]
    pub param: Vec<String>,
    #[arg(long)]
    pub arrange: Vec<String>,
    #[arg(long)]
    pub label: Vec<String>,
    #[arg(long)]
    pub certification: Vec<String>,
    #[arg(long)]
    pub in_stock: bool,
    #[arg(long)]
    pub discount: bool,
    #[arg(long)]
    pub datasheet: bool,
    #[arg(long)]
    pub domestic: bool,
    #[arg(long)]
    pub min_price: Option<f64>,
    #[arg(long)]
    pub max_price: Option<f64>,
    #[arg(long)]
    pub quantity: Option<u64>,
    #[arg(long, default_value = "")]
    pub second: String,
    #[arg(long, value_enum, default_value = "relevance")]
    pub sort: Sort,
    #[arg(long, default_value_t=1, value_parser=clap::value_parser!(u32).range(1..))]
    pub page: u32,
    #[arg(long, default_value_t=30, value_parser=clap::value_parser!(u32).range(1..=100))]
    pub page_size: u32,
}
impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            catalog: vec![],
            brand: vec![],
            package: vec![],
            param: vec![],
            arrange: vec![],
            label: vec![],
            certification: vec![],
            in_stock: false,
            discount: false,
            datasheet: false,
            domestic: false,
            min_price: None,
            max_price: None,
            quantity: None,
            second: String::new(),
            sort: Sort::Relevance,
            page: 1,
            page_size: 30,
        }
    }
}
pub fn default_query(keyword: &str) -> Value {
    json!({"currentPage":1,"pageSize":30,"catalogIdFilter":"","brandIdFilter":"","standardFilter":"","arrangeFilter":"","labelFilter":"","authenticationFilter":"","keyword":keyword,"sortNumber":0,"satisfyStockType":"","startPrice":"","endPrice":"","demandNumber":"","spotFilter":1,"discountFilter":1,"hasDataFile":false,"brandPlaceFilter":"","secondKeyword":"","queryParameterValue":"","lastParamName":""})
}
pub fn api_result(raw: &Value) -> Result<&Value> {
    let code = raw
        .get("code")
        .ok_or_else(|| Error::new("schema_changed", "接口缺少 code 字段"))?;
    if id_string(code) != "200" {
        let e = Error::new(
            "upstream_error",
            format!(
                "商城接口返回 {code}：{}",
                raw["msg"]
                    .as_str()
                    .or(raw["message"].as_str())
                    .unwrap_or("查询失败")
            ),
        );
        return Err(if ["401", "403"].contains(&id_string(code).as_str()) {
            e.fallback()
        } else {
            e
        });
    }
    raw.get("result")
        .ok_or_else(|| Error::new("schema_changed", "接口缺少 result 字段"))
}
pub fn search_result(raw: &Value) -> Result<&Value> {
    let result = &api_result(raw)?["searchResult"];
    if !result["productRecordList"].is_array() {
        return Err(Error::new(
            "schema_changed",
            "搜索响应缺少 searchResult.productRecordList",
        ));
    }
    Ok(result)
}
pub fn id_string(v: &Value) -> String {
    v.as_str().map(str::to_owned).unwrap_or_else(|| {
        if v.is_null() {
            String::new()
        } else {
            v.to_string()
        }
    })
}
pub fn plain_text(text: &str) -> String {
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
    let mut text = TAG.replace_all(text, "").into_owned();
    for (from, to) in [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&#39;", "'"),
        ("&nbsp;", " "),
        ("&amp;", "&"),
    ] {
        text = text.replace(from, to);
    }
    text
}
fn clean(v: &Value) -> Value {
    v.as_str()
        .map(|s| json!(plain_text(s)))
        .unwrap_or(v.clone())
}
fn array(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub fn facets(result: &Value) -> Value {
    let mut result_groups = json!({});
    for (key, field) in [
        ("catalogs", "catalogGroup"),
        ("brands", "gradePlateGroup"),
        ("packages", "encapsulationGroup"),
        ("arrangements", "arrangeGroup"),
        ("labels", "labelGroup"),
        ("certifications", "authenticationGroup"),
    ] {
        result_groups[key] = json!(array(&result[field]));
    }
    result_groups
}
pub fn normalize_product(record: &Value) -> Result<Value> {
    let p = record.get("productVO").unwrap_or(record);
    let code = p["productCode"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::new("schema_changed", "商品缺少 C 编号"))?;
    let id = id_string(&p["productId"]);
    if id.is_empty() {
        return Err(Error::new("schema_changed", "商品缺少 productId"));
    }
    let mut parameters = json!({});
    if let Some(map) = record["paramLinkedMap"].as_object() {
        for (k, v) in map {
            parameters[plain_text(k)] = clean(v);
        }
    }
    let prices: Vec<_> = array(&p["productPriceList"]).iter().map(|x| json!({"quantity":x.get("startPurchasedNumber").or(x.get("spNumber")),"until":x.get("endPurchasedNumber").or(x.get("epNumber")),"unit_price":x["productPrice"]})).collect();
    let mut files = vec![];
    for group in array(&p["fileTypeVOList"]) {
        for file in array(&group["detailVOList"]) {
            let url = file["fileUrl"]
                .as_str()
                .filter(|s| !s.is_empty())
                .and_then(|s| {
                    url::Url::parse("https://atta.szlcsc.com")
                        .ok()?
                        .join(s)
                        .ok()
                })
                .map(|u| u.to_string())
                .or_else(|| file["linkAddress"].as_str().map(str::to_owned));
            if group["fileType"] == "pdf_property" {
                files.push(json!({"type":group["fileType"],"name":file["fileName"],"url":url}));
            }
        }
    }
    Ok(
        json!({"code":code,"id":id,"model":clean(&p["productModel"]),"brand":clean(p.get("productGradePlateNameWithoutHighlight").filter(|v| !v.is_null()).unwrap_or(&p["productGradePlateName"])),"brand_id":id_string(&p["productGradePlateId"]),"package":clean(&p["encapsulationModel"]),"catalog":clean(&p["productType"]),"catalog_id":id_string(&p["productTypeCode"]),"description":clean(&p["productName"]),"intro":clean(&p["remark"]),"stock":record.get("totalStockNumber").filter(|v| !v.is_null()).unwrap_or(&p["stockNumber"]),"min_order":p["minBuyNumber"],"unit":p["productUnit"],"currency":"CNY","prices":prices,"parameters":parameters,"datasheets":files,"url":format!("https://item.szlcsc.com/{id}.html")}),
    )
}
pub fn normalize_detail(props: &Value, summary: Option<&Value>) -> Result<Value> {
    let web = &props["webData"];
    let p = &web["productRecord"];
    let list = web["paramList"]
        .as_array()
        .ok_or_else(|| Error::new("schema_changed", "详情缺少完整参数列表"))?;
    let mut result = normalize_product(p)?;
    result["catalog_id"] = json!(id_string(&web["currentCatalog"]["catalogId"]));
    if !web["currentCatalog"]["catalogName"].is_null() {
        result["catalog"] = web["currentCatalog"]["catalogName"].clone();
    }
    result["stock"] = web
        .get("totalStockNumber")
        .filter(|v| !v.is_null())
        .unwrap_or(&p["stockNumber"])
        .clone();
    result["stock_breakdown"] = json!({"total":web["totalStockNumber"],"guangdong":web["gdWarehouseStockNumber"],"jiangsu":web["jsWarehouseStockNumber"],"smt":web["smtStockNumber"],"record":p["stockNumber"]});
    result["parameters"] = json!({});
    let mut normalized = vec![];
    for item in list {
        let name = item["parameterName"]
            .as_str()
            .ok_or_else(|| Error::new("schema_changed", "参数缺少名称"))?;
        let value = item
            .get("parameterDetailValue")
            .filter(|v| !v.is_null())
            .unwrap_or(&item["parameterValue"]);
        result["parameters"][name] = value.clone();
        normalized.push(json!({"code":item["parameterCode"],"name":name,"value":value}));
    }
    result["parameter_list"] = json!(normalized);
    if let Some(summary) = summary {
        result["prices"] = summary["prices"].clone();
    }
    result["display_price"] = props["price"].clone();
    result["images"] = json!(
        p["luceneBreviaryImageUrls"]
            .as_str()
            .unwrap_or("")
            .split("<$>")
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    );
    result["eda_url"] = p["lcedaProjectUrl"].clone();
    result["eccn"] = p["eccnCode"].clone();
    result["features"] = props["productPdfDesc"]["productPDFDescVO"].clone();
    Ok(result)
}
pub fn parameter_options(aggregation: &Value) -> Result<Value> {
    if aggregation.is_null() {
        return Ok(Value::Null);
    }
    let names = aggregation["paramNameMap"]
        .as_object()
        .ok_or_else(|| Error::new("schema_changed", "属性聚合缺少 paramNameMap"))?;
    if !aggregation["paramGroupMap"].is_object() {
        return Err(Error::new("schema_changed", "属性聚合缺少 paramGroupMap"));
    }
    Ok(json!(names.iter().map(|(code,name)| json!({"code":code,"name":name,"info":aggregation["paramInfoMap"][code],"values":array(&aggregation["paramGroupMap"][code])})).collect::<Vec<_>>()))
}
pub fn resolve_parameters(inputs: &[String], options: &Value) -> Result<Vec<(String, String)>> {
    inputs
        .iter()
        .map(|input| {
            let (name, value) = input
                .split_once('=')
                .filter(|(n, v)| !n.is_empty() && !v.is_empty())
                .ok_or_else(|| Error::invalid("--param 格式为 参数名=值"))?;
            let choices: Vec<_> = array(options)
                .iter()
                .filter(|x| x["code"] == name || x["name"] == name)
                .collect();
            if choices.len() != 1 {
                return Err(Error::invalid(format!(
                    "参数不存在或不唯一：{name}；请查询 filters --catalog"
                )));
            }
            if !array(&choices[0]["values"])
                .iter()
                .any(|x| x["name"] == value)
            {
                return Err(Error::invalid(format!(
                    "参数 {name} 没有值 {value}；请使用 filters 返回的原始值"
                )));
            }
            Ok((id_string(&choices[0]["code"]), value.into()))
        })
        .collect()
}
pub fn encode_parameters(pairs: &[(String, String)]) -> String {
    let input = pairs
        .iter()
        .map(|(code, value)| format!("{value}@{code}"))
        .collect::<Vec<_>>()
        .join("$");
    let mut output = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            output.push(b as char);
        } else {
            output.push_str(&format!("%{b:02X}"));
        }
    }
    output
}
fn numeric_ids(inputs: &[String]) -> bool {
    inputs.iter().all(|s| {
        s.split('$')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    })
}
pub fn resolve_facet(inputs: &[String], choices: &Value, kind: &str) -> Result<String> {
    inputs
        .iter()
        .flat_map(|s| s.split('$'))
        .map(|input| {
            let found: Vec<_> = array(choices)
                .iter()
                .filter(|x| id_string(&x["value"]) == input || x["label"] == input)
                .collect();
            if found.len() == 1 {
                return Ok(id_string(&found[0]["value"]));
            }
            if found.len() > 1 {
                return Err(Error::invalid(format!(
                    "{kind}名称不唯一：{input}，请提供 ID"
                )));
            }
            if !input.is_empty() && input.bytes().all(|b| b.is_ascii_digit()) {
                return Ok(input.into());
            }
            Err(Error::invalid(format!(
                "找不到{kind}选项：{input}，请查询 filters"
            )))
        })
        .collect::<Result<Vec<_>>>()
        .map(|v| v.join("$"))
}
pub enum ItemIdentifier {
    Code(String),
    Id(String),
}
pub fn parse_identifier(input: &str) -> Result<ItemIdentifier> {
    let upper = input.to_ascii_uppercase();
    if let Some(number) = upper.strip_prefix('C')
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
    {
        return Ok(ItemIdentifier::Code(upper));
    }
    if !input.is_empty() && input.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(ItemIdentifier::Id(input.into()));
    }
    if let Ok(url) = url::Url::parse(input)
        && url.origin().ascii_serialization() == "https://item.szlcsc.com"
        && url.username().is_empty()
        && url.password().is_none()
        && let Some(number) = url
            .path()
            .strip_prefix('/')
            .and_then(|s| s.strip_suffix(".html"))
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
    {
        return Ok(ItemIdentifier::Id(number.into()));
    }
    Err(Error::invalid(
        "详情参数必须是 C 编号、商品数字 ID 或 https://item.szlcsc.com/数字.html",
    ))
}
pub fn extract_next_data(html: &str) -> Result<Value> {
    static SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?is)<script\b[^>]*\bid\s*=\s*["']__NEXT_DATA__["'][^>]*>(.*?)</script\s*>"#)
            .unwrap()
    });
    let data = SCRIPT.captures(html).ok_or_else(|| {
        Error::new("page_challenge", "页面没有商品数据，可能是站点校验页").fallback()
    })?;
    let raw: Value = serde_json::from_str(&data[1])?;
    raw.pointer("/props/pageProps")
        .cloned()
        .ok_or_else(|| Error::new("schema_changed", "页面缺少 props.pageProps"))
}
pub struct LcscClient {
    pub http: Http,
    backend: Backend,
    browser: Option<Browser>,
    browser_path: Option<PathBuf>,
    proxy: Option<String>,
    timeout: Duration,
    transport: &'static str,
}
impl LcscClient {
    pub fn new(
        backend: Backend,
        timeout: Duration,
        browser_path: Option<PathBuf>,
        proxy: Option<&str>,
    ) -> Result<Self> {
        Ok(Self {
            http: Http::new(timeout, proxy)?,
            backend,
            browser: None,
            browser_path,
            proxy: proxy.map(str::to_owned),
            timeout,
            transport: "http",
        })
    }
    fn browser(&mut self) -> Result<&mut Browser> {
        if self.browser.is_none() {
            self.browser = Some(Browser::new(
                self.browser_path.clone(),
                self.timeout,
                self.proxy.as_deref(),
            )?);
        }
        self.transport = "browser";
        Ok(self.browser.as_mut().unwrap())
    }
    fn api(&mut self, path: &str, body: &Value) -> Result<Value> {
        if self.backend == Backend::Browser {
            return self.browser()?.api(path, body);
        }
        self.transport = "http";
        let result = self
            .http
            .json(&format!("https://so.szlcsc.com{path}"), Some(body))
            .and_then(|raw| {
                api_result(&raw)?;
                Ok(raw)
            });
        match result {
            Err(e) if self.backend == Backend::Auto && e.fallback => {
                self.browser()?.api(path, body)
            }
            result => result,
        }
    }
    fn raw_search(&mut self, q: &Value) -> Result<Value> {
        let raw = self.api("/query/product", q)?;
        search_result(&raw)?;
        Ok(raw)
    }
    fn aggregate(&mut self, q: &Value) -> Result<Value> {
        let raw = self.api("/query/param/agg?sign=query/product", q)?;
        Ok(api_result(&raw)?.clone())
    }
    pub fn prepare(&mut self, keyword: &str, o: &SearchOptions) -> Result<Value> {
        if keyword.trim().is_empty() {
            return Err(Error::invalid("需要非空关键词"));
        }
        if o.page == 0 || !(1..=100).contains(&o.page_size) || o.quantity == Some(0) {
            return Err(Error::invalid("分页或需求数量无效"));
        }
        if [o.min_price, o.max_price]
            .into_iter()
            .flatten()
            .any(|p| !p.is_finite() || p < 0.)
            || matches!((o.min_price,o.max_price),(Some(a),Some(b)) if a>b)
        {
            return Err(Error::invalid(
                "价格必须为非负有限数字，最低价不能大于最高价",
            ));
        }
        let mut q = default_query(keyword);
        q["currentPage"] = json!(o.page);
        q["pageSize"] = json!(o.page_size);
        q["sortNumber"] = json!(o.sort.number());
        for (k, v) in [
            ("standardFilter", &o.package),
            ("arrangeFilter", &o.arrange),
            ("labelFilter", &o.label),
            ("authenticationFilter", &o.certification),
        ] {
            q[k] = json!(v.join("$"));
        }
        q["spotFilter"] = json!(if o.in_stock { 0 } else { 1 });
        q["discountFilter"] = json!(if o.discount { 0 } else { 1 });
        q["hasDataFile"] = json!(o.datasheet);
        q["brandPlaceFilter"] = json!(if o.domestic {
            "中国大陆$中国台湾"
        } else {
            ""
        });
        q["secondKeyword"] = json!(o.second);
        if let Some(v) = o.min_price {
            q["startPrice"] = json!(v);
        }
        if let Some(v) = o.max_price {
            q["endPrice"] = json!(v);
        }
        if let Some(v) = o.quantity {
            q["demandNumber"] = json!(v);
        }
        if numeric_ids(&o.catalog) {
            q["catalogIdFilter"] = json!(o.catalog.join("$"));
        }
        if numeric_ids(&o.brand) {
            q["brandIdFilter"] = json!(o.brand.join("$"));
        }
        if !numeric_ids(&o.catalog) || !numeric_ids(&o.brand) {
            let mut base = default_query(keyword);
            base["catalogIdFilter"] = q["catalogIdFilter"].clone();
            let raw = self.raw_search(&base)?;
            let groups = facets(search_result(&raw)?);
            q["catalogIdFilter"] = json!(resolve_facet(&o.catalog, &groups["catalogs"], "类目")?);
            q["brandIdFilter"] = json!(resolve_facet(&o.brand, &groups["brands"], "品牌")?);
        }
        if !o.param.is_empty() {
            if q["catalogIdFilter"] == "" {
                let raw = self.raw_search(&q)?;
                let groups = facets(search_result(&raw)?);
                let catalogs = array(&groups["catalogs"]);
                if catalogs.len() != 1 {
                    return Err(Error::invalid("属性筛选需要选择单个类目：--catalog ID"));
                }
                q["catalogIdFilter"] = json!(id_string(&catalogs[0]["value"]));
            }
            if q["catalogIdFilter"].as_str().unwrap_or("").contains('$') {
                return Err(Error::invalid("属性筛选只支持单个类目"));
            }
            let options = parameter_options(&self.aggregate(&q)?)?;
            if options.is_null() {
                return Err(Error::new(
                    "params_unavailable",
                    "未返回属性选项，请检查类目或关键词",
                ));
            }
            let pairs = resolve_parameters(&o.param, &options)?;
            q["queryParameterValue"] = json!(encode_parameters(&pairs));
            q["lastParamName"] = json!(pairs.last().unwrap().0);
        }
        Ok(q)
    }
    pub fn search(&mut self, keyword: &str, options: &SearchOptions) -> Result<Value> {
        let query = self.prepare(keyword, options)?;
        let raw = self.raw_search(&query)?;
        let s = search_result(&raw)?;
        let products = array(&s["productRecordList"])
            .iter()
            .map(normalize_product)
            .collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"source":"szlcsc.com","fetched_at":crate::fetched_at(),"transport":self.transport,"query":query,"page":s["currePage"],"page_size":s["pageSize"],"total":s["totalCount"],"pages":s["countPage"],"products":products,"facets":facets(s),"raw":raw}),
        )
    }
    pub fn filters(&mut self, keyword: &str, options: &SearchOptions) -> Result<Value> {
        let result = self.search(keyword, options)?;
        let mut q = result["query"].clone();
        if q["catalogIdFilter"] == "" && array(&result["facets"]["catalogs"]).len() == 1 {
            q["catalogIdFilter"] = json!(id_string(&result["facets"]["catalogs"][0]["value"]));
        }
        let single = q["catalogIdFilter"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && !s.contains('$'));
        let aggregation = if single {
            self.aggregate(&q)?
        } else {
            Value::Null
        };
        let mut output = result["facets"].clone();
        for (k, v) in [
            ("source", json!("szlcsc.com")),
            ("fetched_at", json!(crate::fetched_at())),
            ("transport", json!(self.transport)),
            ("query", q),
            ("total", result["total"].clone()),
            ("parameters", parameter_options(&aggregation)?),
            (
                "parameter_status",
                json!(if !single {
                    "select_one_catalog"
                } else if aggregation.is_null() {
                    "not_returned"
                } else {
                    "available"
                }),
            ),
            (
                "raw",
                json!({"search":result["raw"],"aggregation":aggregation}),
            ),
        ] {
            output[k] = v;
        }
        Ok(output)
    }
    pub fn detail(&mut self, input: &str) -> Result<Value> {
        let parsed = parse_identifier(input)?;
        let mut summary = None;
        let (id, expected) = match parsed {
            ItemIdentifier::Id(id) => (id, None),
            ItemIdentifier::Code(code) => {
                let search = self.search(&code, &SearchOptions::default())?;
                let found = array(&search["products"])
                    .iter()
                    .find(|p| p["code"] == code)
                    .cloned()
                    .ok_or_else(|| Error::new("not_found", format!("没有找到 {code}")))?;
                let id = id_string(&found["id"]);
                summary = Some(found);
                (id, Some(code))
            }
        };
        let url = format!("https://item.szlcsc.com/{id}.html");
        self.transport = "http";
        let props = if self.backend == Backend::Browser {
            self.browser()?.page(&url)?
        } else {
            let result = self
                .http
                .get(&url)
                .and_then(|b| extract_next_data(&String::from_utf8_lossy(&b)));
            match result {
                Err(e) if self.backend == Backend::Auto && e.fallback => {
                    self.browser()?.page(&url)?
                }
                other => other?,
            }
        };
        let product = normalize_detail(&props, summary.as_ref())?;
        if expected.is_some_and(|code| product["code"] != code) || product["id"] != id {
            return Err(Error::new("identity_mismatch", "商品页面与查询标识不一致"));
        }
        Ok(
            json!({"source":"szlcsc.com","fetched_at":crate::fetched_at(),"transport":self.transport,"product":product,"raw":props}),
        )
    }
    pub fn catalogs(&self, keyword: &str) -> Result<Value> {
        let raw = self.http.json(
            "https://staticdata.szlcsc.com/common/data/resource-catalog-list-new.json",
            None,
        )?;
        api_result(&raw)?;
        let mut categories = vec![];
        visit_catalogs(&raw, &[], keyword, &mut categories);
        if categories.is_empty() && keyword.is_empty() {
            return Err(Error::new("schema_changed", "类目数据格式变化"));
        }
        Ok(
            json!({"source":"szlcsc.com","fetched_at":crate::fetched_at(),"categories":categories,"raw":raw}),
        )
    }
}
fn visit_catalogs(node: &Value, path: &[String], keyword: &str, out: &mut Vec<Value>) {
    if let Some(list) = node.as_array() {
        for child in list {
            visit_catalogs(child, path, keyword, out);
        }
    }
    if let Some(map) = node.as_object() {
        let mut path = path.to_vec();
        if let Some(name) = node["catalogName"].as_str() {
            path.push(name.into());
            if !node["catalogId"].is_null() && path.join(" / ").contains(keyword) {
                out.push(
                    json!({"id":id_string(&node["catalogId"]),"name":name,"path":path.join(" / ")}),
                );
            }
        }
        for child in map.values().filter(|v| v.is_array() || v.is_object()) {
            visit_catalogs(child, &path, keyword, out);
        }
    }
}
pub fn without_raw(value: &Value) -> Value {
    match value {
        Value::Array(a) => json!(a.iter().map(without_raw).collect::<Vec<_>>()),
        Value::Object(o) => Value::Object(
            o.iter()
                .filter(|(k, _)| *k != "raw")
                .map(|(k, v)| (k.clone(), without_raw(v)))
                .collect(),
        ),
        _ => value.clone(),
    }
}
