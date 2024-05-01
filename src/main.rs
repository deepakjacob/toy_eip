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
            "the scheme of url {} - source is {}",
            &url.scheme(),
            &location,
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
            "the scheme of url {} - destination is {}",
            &url.scheme(),
            &location,
        );
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
            // verify that the destination is provided for the route
            self.verify_route_destination(route);
            let source = &route.source;
            let source_scheme = (&route.source).scheme();
            if source_scheme == "file" {
                let (rx, mut watcher) = self.get_plaform_file_listener();
                self.file_paths.push(source.path().to_string());
                debug!(
                    "added folder {} to the watch list",
                    source.path().to_string()
                );

                watcher
                    .watch(Path::new(source.path()), notify::RecursiveMode::Recursive)
                    .unwrap();
            }
        }

        Ok(())
    }

    fn run_for_file(&mut self) -> Result<(), io::Error> {
        for folder in &self.file_paths {}
        Ok(loop {
            match rx.recv() {
                Ok(event) => {
                    if let Some(file_name) = event.paths.get(0) {
                        debug!("Received: {:?}", file_name);

                        {
                            // let extension_filter = &route.extension.clone().unwrap_or_else(|| "*".to_string());
                            // let destination = &route.destination;

                            let mut file = File::open(file_name)?;
                            let mut contents = Vec::new();
                            file.read_to_end(&mut contents)?;
                            debug!("the size of the contents:      {}", &contents.len());

                            let mut transformed_data = TransformedData { data: contents };

                            // for transformer in &route.transformers {
                            //     transformed_data = transformer.transform(&transformed_data.data)?;
                            // }

                            // let output_path = destination.join(entry.file_name());
                            // let mut output_file = BufWriter::new(File::create(output_path)?);
                            // output_file.write_all(&transformed_data.data)?;
                        }
                    }
                }
                Err(e) => {
                    debug!("Receive error: {:?}", e);
                    break;
                }
            }
        })
    }

    fn verify_route_destination(&self, route: &Route) {
        if let None = route.destination {
            panic!(
                "for the route with from {}, no to(\"uri\") has been found! ",
                route.source
            );
        }
    }

    fn get_plaform_file_listener(&self) -> (Receiver<Event>, INotifyWatcher) {
        let (tx, rx) = channel();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| match res {
                Ok(event) => match event.kind {
                    notify::EventKind::Create(_) => {
                        if let Err(e) = tx.send(event) {
                            debug!("error sending event for {:?}", e);
                        }
                    }
                    _ => {}
                },
                Err(e) => debug!("err       {:?}", e),
            })
            .unwrap();
        (rx, watcher)
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
    debug!("output directory url  {:?}", &output_dir_url);

    let toy_eip = EIP::new();
    toy_eip
        // .routes(vec![Route::start(&input_dir_url.as_str())
        .routes(vec![Route::start(&input_dir_url.as_str())
            .extension("csv")
            .stream()
            .then(transformer)
            .to(&output_dir_url.as_str())])
        .run()
}
