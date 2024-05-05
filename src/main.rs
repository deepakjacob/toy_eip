use core::panic;
use std::{
    env,
    fs::File,
    io::{self, Read, Write},
    path::Path,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
};

use log::debug;
use notify::{Event, INotifyWatcher, Watcher};
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
        Self { chunk_size }
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
        let url = Url::parse(location).expect("Invalid source URL");
        debug!("URL scheme for source {}: {}", location, url.scheme());

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
        let url = Url::parse(location).expect("Invalid destination URL");
        debug!("URL scheme for destination {}: {}", location, url.scheme());
        self.destination = Some(url);
        self
    }
}

struct EIP {
    routes: Vec<Route>,
    file_paths: Vec<String>,
}

impl EIP {
    fn new() -> Self {
        Self {
            routes: Vec::new(),
            file_paths: Vec::new(),
        }
    }

    fn routes(mut self, r: Vec<Route>) -> Self {
        self.routes = r;
        self
    }

    fn run(&mut self) -> io::Result<()> {
        for route in &self.routes {
            self.verify_route_destination(route);
            if route.source.scheme() == "file" {
                let (rx, mut watcher) = get_plaform_file_listener();
                self.file_paths.push(route.source.path().to_string());
                debug!("Added {} to watch list", route.source.path());
                add_to_file_watcher(watcher, &route.source);
                self.run_for_file(rx, route)?;
            }
        }
        Ok(())
    }

    fn run_for_file(&self, rx: Receiver<Event>, route: &Route) -> io::Result<()> {
        loop {
            match rx.recv() {
                Ok(event) => {
                    if let Some(file_name) = event.paths.get(0) {
                        debug!("Received file event: {:?}", file_name);

                        let mut file = File::open(file_name)?;
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents)?;
                        debug!("File size: {}", contents.len());

                        let mut transformed_data = TransformedData { data: contents };
                        for transformer in &route.transformers {
                            transformed_data = transformer.transform(&transformed_data.data)?;
                        }
                    }
                }
                Err(e) => {
                    debug!("Receive error: {:?}", e);
                    return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
                }
            }
        }
    }

    fn verify_route_destination(&self, route: &Route) {
        if route.destination.is_none() {
            panic!("No destination found for the route from {}", route.source);
        }
    }
}

fn add_to_file_watcher(mut watcher: INotifyWatcher, source: &Url) {
    watcher
        .watch(Path::new(source.path()), notify::RecursiveMode::Recursive)
        .unwrap();
}

fn get_plaform_file_listener() -> (Receiver<Event>, INotifyWatcher) {
    let (tx, rx) = channel();
    let watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| match res {
            Ok(event) => {
                if matches!(event.kind, notify::EventKind::Create(_)) {
                    if let Err(e) = tx.send(event) {
                        debug!("Error sending event: {:?}", e);
                    }
                }
            }
            Err(e) => debug!("Error receiving event: {:?}", e),
        })
        .unwrap();
    (rx, watcher)
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

    let transformer = Arc::new(FileTransformer::new(1024));

    let current_dir = env::current_dir()?;
    debug!("Current directory: {:?}", &current_dir);

    let input_dir_url = Url::from_directory_path(current_dir.join("input")).unwrap();
    debug!("Input directory URL: {}", &input_dir_url);

    let output_dir_url = Url::from_directory_path(current_dir.join("output")).unwrap();
    debug!("Output directory URL: {}", &output_dir_url);

    EIP::new()
        .routes(vec![Route::start(input_dir_url.as_str())
            .extension("csv")
            .stream()
            .then(transformer)
            .to(output_dir_url.as_str())])
        .run()
}
