use core::panic;
use std::{
    env,
    io::{self, Write},
    sync::Arc,
};

use log::debug;
use url::Url;

struct TransformedData {
    data: Vec<u8>,
}

trait Transformer {
    fn transform(&self, input: &[u8]) -> io::Result<TransformedData>;
}

struct FileTransformer {
    chunk_size: usize,
}

impl FileTransformer {
    fn new(chunk_size: usize) -> Self {
        FileTransformer { chunk_size }
    }
}

impl Transformer for FileTransformer {
    fn transform(&self, input: &[u8]) -> io::Result<TransformedData> {
        let mut data = Vec::new();
        let mut start = 0;
        while start < input.len() {
            let end = std::cmp::min(start + self.chunk_size, input.len());
            data.extend_from_slice(&input[start..end]);
            start = end;
        }
        Ok(TransformedData { data })
    }
}

trait Consumer {
    fn consume(&self, route: &Route) -> io::Result<Vec<u8>>;
}
struct FileConsumer;

impl Consumer for FileConsumer {
    fn consume(&self, route: &Route) -> io::Result<Vec<u8>> {
        todo!()
    }
}

struct Route {
    source: Url,
    extension: Option<String>,
    transformers: Vec<Arc<dyn Transformer>>,
    destination: Option<Url>,
}

impl Route {
    fn start(location: &str) -> Self {
        let url = Url::parse(location).expect("the source {location} is not valid");
        debug!(
            "{}",
            format!(
                "the scheme of url {} - source is {}",
                &url.scheme(),
                &location,
            )
        );
        if url.scheme() == "file" {
            debug!("need to switch on folder listeners !");
        }

        Self {
            source: url,
            extension: None,
            transformers: Vec::new(),
            destination: None,
        }
    }

    fn extension(mut self, ext: &str) -> Self {
        self.extension = Some(ext.to_string());
        self
    }

    fn stream(self) -> Self {
        self
    }

    fn then(mut self, transformer: Arc<dyn Transformer>) -> Self {
        self.transformers.push(transformer);
        self
    }

    fn to(mut self, location: &str) -> Self {
        let url = Url::parse(location).expect("the destination {location} is not valid");
        debug!(
            "{}",
            format!(
                "the scheme of url {} - destination is {}",
                &url.scheme(),
                &location,
            )
        );
        self.destination = Some(url);
        self
    }
}

struct EIP {
    routes: Vec<Route>,
}

impl EIP {
    fn new() -> Self {
        Self { routes: Vec::new() }
    }

    fn routes(mut self, r: Vec<Route>) -> Self {
        self.routes = r;
        self
    }

    fn run(&self) -> io::Result<()> {
        for route in &self.routes {
            if let None = &route.destination {
                panic!(
                    "for the route with from {}, no to(\"uri\") has been found! ",
                    &route.source
                );
            }
            // let extension_filter = &route.extension.clone().unwrap_or_else(|| "*".to_string());
            // let destination = &route.destination;

            // for entry in WalkDir::new(&route.location)
            //     .into_iter()
            //     .filter_map(Result::ok)
            //     .filter(|e| e.file_type().is_file())
            //     .filter(|e| {
            //         e.path()
            //             .extension()
            //             .and_then(|ext| ext.to_str())
            //             .map(|ext| ext == extension_filter.trim_start_matches('*'))
            //             .unwrap_or(false)
            //     })
            // {
            //     let mut file = File::open(entry.path())?;
            //     let mut contents = Vec::new();
            //     file.read_to_end(&mut contents)?;
            //     debug!("the size of the contents:      {}", &contents.len());

            //     let mut transformed_data = TransformedData { data: contents };

            //     for transformer in &route.transformers {
            //         transformed_data = transformer.transform(&transformed_data.data)?;
            //     }

            //     let output_path = destination.join(entry.file_name());
            //     let mut output_file = BufWriter::new(File::create(output_path)?);
            //     output_file.write_all(&transformed_data.data)?;
            // }
        }

        Ok(())
    }
}

fn main() -> io::Result<()> {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}:{}][{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .init();

    //TODO: to be removed
    let transformer = Arc::new(FileTransformer::new(1024));

    let current_dir = env::current_dir().expect("failed to get current directory");
    debug!("current directory     {:?}", &current_dir);

    let input_dir_url =
        Url::from_file_path(current_dir.join("input").canonicalize().unwrap()).unwrap();
    debug!("input directory url   {:?}", &input_dir_url.as_str());

    let output_dir_url =
        Url::from_file_path(current_dir.join("output").canonicalize().unwrap()).unwrap();
    debug!("input directory url   {:?}", &output_dir_url);

    let toy_eip = EIP::new();
    toy_eip
        .routes(vec![Route::start(&input_dir_url.as_str())
            .extension("csv")
            .stream()
            .then(transformer)
            .to(&output_dir_url.as_str())])
        .run()
}
