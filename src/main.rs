use clap::{Parser, Subcommand};
use fantasy_lcsc::{
    Error, Result,
    http::Http,
    lcsc::{Backend, LcscClient, SearchOptions, without_raw},
    models::{ModelClient, normalize_parts},
};
use serde_json::{Value, json};
use std::{io::Write, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(
    version,
    about = "立创商城国内站查询与 EasyEDA 原始 STEP 下载",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// 输出结构化 JSON，错误写入 stderr
    #[arg(long, global = true)]
    json: bool,
    /// 输出来源原始响应（download 不支持）
    #[arg(long, global = true)]
    raw: bool,
    /// 同时保存 JSON 到文件
    #[arg(long, global = true)]
    out: Option<PathBuf>,
    #[arg(long, global = true, value_enum, default_value = "auto")]
    backend: Backend,
    /// 请求超时秒数
    #[arg(long,global=true,default_value_t=30,value_parser=clap::value_parser!(u64).range(1..=300))]
    timeout: u64,
    #[arg(long, global = true, env = "LCSC_BROWSER_PATH")]
    browser_path: Option<PathBuf>,
    /// 显式代理 URL；默认直连
    #[arg(long, global = true, env = "FANTASY_LCSC_PROXY")]
    proxy: Option<String>,
}
#[derive(Subcommand)]
enum Commands {
    /// 检查远端更新，或后台更新已安装的 CLI 与 EDA skill
    Update {
        /// 只检查远端，不替换已安装文件
        #[arg(long, conflicts_with = "local")]
        check: bool,
        /// 使用本地源码重建，不拉取远端
        #[arg(long)]
        local: bool,
    },
    /// 搜索元件、组合筛选和分页（人民币）
    Search {
        keyword: String,
        #[command(flatten)]
        options: SearchOptions,
    },
    /// 查询类目、品牌、封装与动态参数选项
    Filters {
        keyword: String,
        #[command(flatten)]
        options: SearchOptions,
    },
    /// 读取完整参数、阶梯价、规格书与各仓库存
    Detail {
        #[arg(required=true,num_args=1..)]
        identifiers: Vec<String>,
    },
    /// 查找国内商城类目 ID
    Catalogs {
        #[arg(default_value = "")]
        keyword: String,
    },
    /// 查询 EasyEDA 元件、封装与关联模型 UUID（保留来源）
    Info {
        #[arg(required=true,num_args=1..)]
        parts: Vec<String>,
    },
    /// 批量下载原始 STEP；单个失败仍继续
    Download {
        #[arg(num_args=0..)]
        parts: Vec<String>,
        /// 从 UTF-8 文本读编号，空格/逗号/分号分隔
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(short = 'o', long, default_value = "step")]
        output: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
}
fn run(cli: &Cli) -> Result<(Value, bool)> {
    let timeout = Duration::from_secs(cli.timeout);
    match &cli.command {
        Commands::Update { check, local } => {
            if cli.raw {
                return Err(Error::invalid("update 不支持 --raw，请使用 --json"));
            }
            Ok((fantasy_lcsc::maintenance::update(*check, *local)?, false))
        }
        Commands::Download {
            parts,
            file,
            output,
            overwrite,
        } => {
            if cli.raw {
                return Err(Error::invalid(
                    "download 不支持 --raw；模型元数据可用 info --raw 查询",
                ));
            }
            let mut values = parts.clone();
            if let Some(path) = file {
                values.push(
                    std::fs::read_to_string(path)?
                        .trim_start_matches('\u{feff}')
                        .into(),
                );
            }
            let parts = normalize_parts(&values)?;
            let client = ModelClient::new(Http::new(timeout, cli.proxy.as_deref())?);
            let output = if output.is_absolute() {
                output.clone()
            } else {
                std::env::current_dir()?.join(output)
            };
            let mut results = vec![];
            let mut failed = 0;
            for part in &parts {
                let mut saved = vec![];
                let result = client.download(part, &output, *overwrite, &mut |event| {
                    if let fantasy_lcsc::models::DownloadEvent::Saved(file) = event {
                        saved.push(file);
                    }
                    true
                });
                match result {
                    Ok(files) => {
                        results.push(json!({"code":part,"ok":true,"files":files}));
                    }
                    Err(error) => {
                        failed += 1;
                        results.push(json!({"code":part,"ok":false,"error":error,"files":saved}));
                    }
                }
                if !cli.json {
                    eprintln!(
                        "[{}/{}] {} {}",
                        results.len(),
                        parts.len(),
                        part,
                        if results.last().unwrap()["ok"] == true {
                            "完成"
                        } else {
                            "失败"
                        }
                    );
                }
            }
            Ok((
                json!({"source":fantasy_lcsc::models::SOURCE,"fetched_at":fantasy_lcsc::fetched_at(),"output":output,"succeeded":parts.len()-failed,"failed":failed,"results":results}),
                failed > 0,
            ))
        }
        Commands::Info { parts } => {
            let parts = normalize_parts(parts)?;
            let client = ModelClient::new(Http::new(timeout, cli.proxy.as_deref())?);
            let mut results = vec![];
            let mut failed = false;
            for part in parts {
                match client.info(&part) {
                    Ok(info) => results.push(info),
                    Err(error) => {
                        failed = true;
                        results.push(json!({"code":part,"error":error}));
                    }
                }
            }
            let raw = Value::Array(
                results
                    .iter()
                    .map(|r| r.get("raw").cloned().unwrap_or(r.clone()))
                    .collect(),
            );
            Ok((
                json!({"source":fantasy_lcsc::models::SOURCE,"components":results,"raw":raw}),
                failed,
            ))
        }
        command => {
            let mut client = LcscClient::new(
                cli.backend,
                timeout,
                cli.browser_path.clone(),
                cli.proxy.as_deref(),
            )?;
            let result = match command {
                Commands::Search { keyword, options } => client.search(keyword, options)?,
                Commands::Filters { keyword, options } => client.filters(keyword, options)?,
                Commands::Catalogs { keyword } => client.catalogs(keyword)?,
                Commands::Detail { identifiers } => {
                    // Validate the whole list before issuing requests.
                    for input in identifiers {
                        fantasy_lcsc::lcsc::parse_identifier(input)?;
                    }
                    if identifiers.len() == 1 {
                        client.detail(&identifiers[0])?
                    } else {
                        let mut products = vec![];
                        let mut raw = vec![];
                        for input in identifiers {
                            let result = client.detail(input)?;
                            products.push(result["product"].clone());
                            raw.push(result["raw"].clone());
                        }
                        json!({"source":"szlcsc.com","fetched_at":fantasy_lcsc::fetched_at(),"products":products,"raw":raw})
                    }
                }
                _ => unreachable!(),
            };
            Ok((result, false))
        }
    }
}
fn render(value: &Value, command: &Commands) -> String {
    fn cell(v: &Value) -> String {
        fantasy_lcsc::lcsc::id_string(v).replace(['\n', '\r', '\t'], " ")
    }
    match command {
        Commands::Search { .. } => {
            let mut out = format!(
                "共 {} 条，第 {}/{} 页（人民币）\n编号\t型号\t品牌\t封装\t库存\t首阶价格\n",
                value["total"], value["page"], value["pages"]
            );
            for p in value["products"].as_array().into_iter().flatten() {
                out.push_str(
                    &[
                        &p["code"],
                        &p["model"],
                        &p["brand"],
                        &p["package"],
                        &p["stock"],
                        &p["prices"][0]["unit_price"],
                    ]
                    .map(cell)
                    .join("\t"),
                );
                out.push('\n');
            }
            out
        }
        Commands::Catalogs { .. } => value["categories"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| format!("{}\t{}", cell(&c["id"]), cell(&c["path"])))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => serde_json::to_string_pretty(value).unwrap_or_default(),
    }
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let json_errors = args.iter().any(|s| s == "--json" || s == "--raw");
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            if code == 2 && json_errors {
                eprintln!("{}", json!({"error":Error::invalid(error.to_string())}));
            } else {
                let _ = error.print();
            }
            std::process::exit(code);
        }
    };
    let result = run(&cli).and_then(|(result, failed)| {
        let value = if cli.raw {
            result.get("raw").cloned().unwrap_or(Value::Null)
        } else {
            without_raw(&result)
        };
        let json = serde_json::to_string_pretty(&value)?;
        if let Some(path) = &cli.out {
            fantasy_lcsc::atomic_write(path, format!("{json}\n").as_bytes(), true)?;
        }
        let output = if cli.json || cli.raw {
            json
        } else {
            render(&value, &cli.command)
        };
        // A closed downstream pipe is normal for CLI pipelines.
        match writeln!(std::io::stdout().lock(), "{output}") {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e.into()),
        }
        Ok(failed)
    });
    match result {
        Ok(failed) => std::process::exit(if failed { 1 } else { 0 }),
        Err(error) => {
            if cli.json || cli.raw {
                eprintln!("{}", json!({"error":error}));
            } else {
                eprintln!("错误 [{}]：{}", error.code, error.message);
            }
            std::process::exit(if error.code == "invalid_argument" {
                2
            } else {
                1
            });
        }
    }
}
