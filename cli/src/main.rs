use vectordb_client::{ClientBuilder, VectorDbClient};
use vectordb_common::types::*;
use clap::{Arg, Command, ArgMatches};
use anyhow::{Result, Context};
use tabled::{Table, Tabled};
use colored::*;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let app = Command::new("vectordb-cli")
        .version("0.1.0")
        .about("Command-line interface for VectorDB")
        .arg(
            Arg::new("endpoint")
                .long("endpoint")
                .short('e')
                .value_name("URL")
                .help("Server endpoint")
                .default_value("http://localhost:9090")
        )
        .arg(
            Arg::new("protocol")
                .long("protocol")
                .short('p')
                .value_name("PROTOCOL")
                .help("Protocol to use (grpc, rest)")
                .default_value("grpc")
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .value_name("SECONDS")
                .help("Request timeout in seconds")
                .default_value("30")
        )
        .subcommand(
            Command::new("collections")
                .about("Collection management")
                .subcommand(
                    Command::new("create")
                        .about("Create a new collection")
                        .arg(Arg::new("name").help("Collection name").required(true))
                        .arg(Arg::new("dimension").long("dimension").short('d').help("Vector dimension").required(true))
                        .arg(Arg::new("metric").long("metric").short('m').help("Distance metric (cosine, euclidean, dot_product, manhattan)").default_value("cosine"))
                )
                .subcommand(
                    Command::new("list")
                        .about("List all collections")
                )
                .subcommand(
                    Command::new("info")
                        .about("Get collection information")
                        .arg(Arg::new("name").help("Collection name").required(true))
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a collection")
                        .arg(Arg::new("name").help("Collection name").required(true))
                        .arg(Arg::new("confirm").long("confirm").help("Skip confirmation prompt").action(clap::ArgAction::SetTrue))
                )
        )
        .subcommand(
            Command::new("vectors")
                .about("Vector operations")
                .subcommand(
                    Command::new("insert")
                        .about("Insert a vector")
                        .arg(Arg::new("collection").help("Collection name").required(true))
                        .arg(Arg::new("vector").help("Vector data as JSON array").required(true))
                        .arg(Arg::new("id").long("id").help("Vector ID (optional)"))
                        .arg(Arg::new("metadata").long("metadata").help("Metadata as JSON object"))
                )
                .subcommand(
                    Command::new("insert-file")
                        .about("Insert vectors from file")
                        .arg(Arg::new("collection").help("Collection name").required(true))
                        .arg(Arg::new("file").help("File path (JSON lines format)").required(true))
                        .arg(Arg::new("batch-size").long("batch-size").help("Batch size for insertion").default_value("100"))
                )
                .subcommand(
                    Command::new("query")
                        .about("Query for nearest neighbors")
                        .arg(Arg::new("collection").help("Collection name").required(true))
                        .arg(Arg::new("vector").help("Query vector as JSON array").required(true))
                        .arg(Arg::new("limit").long("limit").short('l').help("Number of results").default_value("10"))
                        .arg(Arg::new("ef-search").long("ef-search").help("EF search parameter"))
                )
                .subcommand(
                    Command::new("get")
                        .about("Get a vector by ID")
                        .arg(Arg::new("collection").help("Collection name").required(true))
                        .arg(Arg::new("id").help("Vector ID").required(true))
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a vector")
                        .arg(Arg::new("collection").help("Collection name").required(true))
                        .arg(Arg::new("id").help("Vector ID").required(true))
                )
        )
        .subcommand(
            Command::new("stats")
                .about("Get server statistics")
        )
        .subcommand(
            Command::new("health")
                .about("Check server health")
        );

    let matches = app.get_matches();

    // Create client
    let client = create_client(&matches).await?;

    match matches.subcommand() {
        Some(("collections", sub_matches)) => handle_collections_command(&*client, sub_matches).await,
        Some(("vectors", sub_matches)) => handle_vectors_command(&*client, sub_matches).await,
        Some(("stats", _)) => handle_stats_command(&*client).await,
        Some(("health", _)) => handle_health_command(&*client).await,
        _ => {
            println!("{}", "No subcommand provided. Use --help for usage information.".yellow());
            Ok(())
        }
    }
}

async fn create_client(matches: &ArgMatches) -> Result<Box<dyn VectorDbClient>> {
    let endpoint = matches.get_one::<String>("endpoint").unwrap();
    let protocol = matches.get_one::<String>("protocol").unwrap();
    let timeout: u64 = matches.get_one::<String>("timeout").unwrap().parse()?;

    let mut builder = ClientBuilder::new().timeout(timeout);

    builder = match protocol.as_str() {
        "grpc" => builder.grpc(endpoint),
        "rest" => {
            let rest_endpoint = if endpoint.contains(":9090") {
                endpoint.replace(":9090", ":8080")
            } else if !endpoint.contains(":") {
                format!("{}:8080", endpoint)
            } else {
                endpoint.clone()
            };
            builder.rest(rest_endpoint)
        }
        _ => {
            return Err(anyhow::anyhow!("Invalid protocol: {}", protocol));
        }
    };

    builder.build().await.context("Failed to create client")
}

async fn handle_collections_command(client: &dyn VectorDbClient, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("create", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            let dimension: usize = sub_matches.get_one::<String>("dimension").unwrap().parse()?;
            let metric_str = sub_matches.get_one::<String>("metric").unwrap();

            let distance_metric = match metric_str.as_str() {
                "cosine" => DistanceMetric::Cosine,
                "euclidean" => DistanceMetric::Euclidean,
                "dot_product" => DistanceMetric::DotProduct,
                "manhattan" => DistanceMetric::Manhattan,
                _ => return Err(anyhow::anyhow!("Invalid distance metric: {}", metric_str)),
            };

            let config = CollectionConfig {
                name: name.clone(),
                dimension,
                distance_metric,
                vector_type: VectorType::Float32,
                index_config: IndexConfig::default(),
                quantization: None,
            };

            client.create_collection(&config).await?;
            println!("{}", format!("✓ Collection '{}' created successfully", name).green());
        }
        Some(("list", _)) => {
            let collections = client.list_collections().await?;
            
            if collections.is_empty() {
                println!("{}", "No collections found".yellow());
            } else {
                println!("{}", "Collections:".bold());
                for collection in collections {
                    println!("  • {}", collection.cyan());
                }
            }
        }
        Some(("info", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            
            match client.get_collection_info(name).await {
                Ok((config, stats)) => {
                    println!("{}", format!("Collection: {}", config.name).bold());
                    println!("  Dimension: {}", config.dimension);
                    println!("  Distance Metric: {:?}", config.distance_metric);
                    println!("  Vector Type: {:?}", config.vector_type);
                    println!("  Vector Count: {}", stats.vector_count);
                    println!("  Memory Usage: {} bytes", stats.memory_usage);
                    println!("  Index Size: {} bytes", stats.index_size);
                }
                Err(e) => {
                    println!("{}", format!("✗ Failed to get collection info: {}", e).red());
                }
            }
        }
        Some(("delete", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            let confirm = sub_matches.get_flag("confirm");

            if !confirm {
                println!("{}", format!("Are you sure you want to delete collection '{}'? (y/N)", name).yellow());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().to_lowercase().starts_with('y') {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            client.delete_collection(name).await?;
            println!("{}", format!("✓ Collection '{}' deleted successfully", name).green());
        }
        _ => {
            println!("{}", "No collections subcommand provided".yellow());
        }
    }

    Ok(())
}

async fn handle_vectors_command(client: &dyn VectorDbClient, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("insert", sub_matches)) => {
            let collection = sub_matches.get_one::<String>("collection").unwrap();
            let vector_str = sub_matches.get_one::<String>("vector").unwrap();
            let id_str = sub_matches.get_one::<String>("id");
            let metadata_str = sub_matches.get_one::<String>("metadata");

            let vector_data: Vec<f32> = serde_json::from_str(vector_str)?;
            let id = if let Some(id_str) = id_str {
                Uuid::parse_str(id_str)?
            } else {
                Uuid::new_v4()
            };

            let metadata = if let Some(metadata_str) = metadata_str {
                Some(serde_json::from_str(metadata_str)?)
            } else {
                None
            };

            let vector = Vector {
                id,
                data: vector_data,
                metadata,
            };

            client.insert(collection, &vector).await?;
            println!("{}", format!("✓ Vector inserted with ID: {}", id).green());
        }
        Some(("query", sub_matches)) => {
            let collection = sub_matches.get_one::<String>("collection").unwrap();
            let vector_str = sub_matches.get_one::<String>("vector").unwrap();
            let limit: usize = sub_matches.get_one::<String>("limit").unwrap().parse()?;
            let ef_search = sub_matches.get_one::<String>("ef-search").map(|s| s.parse().unwrap());

            let query_vector: Vec<f32> = serde_json::from_str(vector_str)?;

            let request = QueryRequest {
                collection: collection.clone(),
                vector: query_vector,
                limit,
                ef_search,
                filter: None,
            };

            let results = client.query(&request).await?;

            if results.is_empty() {
                println!("{}", "No results found".yellow());
            } else {
                #[derive(Tabled)]
                struct QueryResultTable {
                    #[tabled(rename = "ID")]
                    id: String,
                    #[tabled(rename = "Distance")]
                    distance: String,
                    #[tabled(rename = "Metadata")]
                    metadata: String,
                }

                let table_data: Vec<QueryResultTable> = results
                    .into_iter()
                    .map(|r| QueryResultTable {
                        id: r.id.to_string(),
                        distance: format!("{:.6}", r.distance),
                        metadata: r.metadata.map_or("None".to_string(), |m| {
                            serde_json::to_string(&m).unwrap_or_else(|_| "Invalid".to_string())
                        }),
                    })
                    .collect();

                let table = Table::new(table_data);
                println!("{}", table);
            }
        }
        Some(("get", sub_matches)) => {
            let collection = sub_matches.get_one::<String>("collection").unwrap();
            let id_str = sub_matches.get_one::<String>("id").unwrap();
            let id = Uuid::parse_str(id_str)?;

            match client.get(collection, &id).await? {
                Some(vector) => {
                    println!("{}", format!("Vector ID: {}", vector.id).bold());
                    println!("Data: {:?}", vector.data);
                    if let Some(metadata) = vector.metadata {
                        println!("Metadata: {}", serde_json::to_string_pretty(&metadata)?);
                    }
                }
                None => {
                    println!("{}", format!("Vector with ID {} not found", id).yellow());
                }
            }
        }
        Some(("delete", sub_matches)) => {
            let collection = sub_matches.get_one::<String>("collection").unwrap();
            let id_str = sub_matches.get_one::<String>("id").unwrap();
            let id = Uuid::parse_str(id_str)?;

            let deleted = client.delete(collection, &id).await?;
            if deleted {
                println!("{}", format!("✓ Vector {} deleted successfully", id).green());
            } else {
                println!("{}", format!("Vector {} not found", id).yellow());
            }
        }
        _ => {
            println!("{}", "No vectors subcommand provided".yellow());
        }
    }

    Ok(())
}

async fn handle_stats_command(client: &dyn VectorDbClient) -> Result<()> {
    let stats = client.get_stats().await?;
    
    println!("{}", "Server Statistics".bold());
    println!("  Total Collections: {}", stats.total_collections);
    println!("  Total Vectors: {}", stats.total_vectors);
    println!("  Memory Usage: {} bytes ({:.2} MB)", 
             stats.memory_usage, 
             stats.memory_usage as f64 / 1024.0 / 1024.0);
    println!("  Disk Usage: {} bytes ({:.2} MB)", 
             stats.disk_usage, 
             stats.disk_usage as f64 / 1024.0 / 1024.0);
    println!("  Uptime: {} seconds", stats.uptime_seconds);

    Ok(())
}

async fn handle_health_command(client: &dyn VectorDbClient) -> Result<()> {
    match client.health().await {
        Ok(true) => {
            println!("{}", "✓ Server is healthy".green());
        }
        Ok(false) => {
            println!("{}", "✗ Server is unhealthy".red());
        }
        Err(e) => {
            println!("{}", format!("✗ Health check failed: {}", e).red());
        }
    }

    Ok(())
}