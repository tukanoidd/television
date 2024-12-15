use crate::channels::{OnAir, TelevisionChannel};
use crate::entry::{Entry, PreviewCommand, PreviewType};
use devicons::FileIcon;
use std::collections::HashSet;
use std::path::PathBuf;
use television_fuzzy::matcher::{config::Config, injector::Injector, Matcher};
use television_utils::files::{walk_builder, DEFAULT_NUM_THREADS};

pub struct Channel {
    matcher: Matcher<String>,
    crawl_handle: tokio::task::JoinHandle<()>,
    // PERF: cache results (to make deleting characters smoother) with
    // a shallow stack of sub-patterns as keys (e.g. "a", "ab", "abc")
}

impl Channel {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        let matcher = Matcher::new(Config::default().match_paths(true));
        // start loading files in the background
        let crawl_handle = tokio::spawn(load_dirs(paths, matcher.injector()));
        Channel {
            matcher,
            crawl_handle,
        }
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new(vec![std::env::current_dir().unwrap()])
    }
}

impl From<&mut TelevisionChannel> for Channel {
    fn from(value: &mut TelevisionChannel) -> Self {
        match value {
            c @ TelevisionChannel::GitRepos(_) => {
                let entries = c.results(c.result_count(), 0);
                Self::new(
                    entries
                        .iter()
                        .map(|entry| PathBuf::from(entry.name.clone()))
                        .collect(),
                )
            }
            c @ TelevisionChannel::Dirs(_) => {
                let entries = c.results(c.result_count(), 0);
                Self::new(
                    entries
                        .iter()
                        .map(|entry| PathBuf::from(&entry.name))
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect(),
                )
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(unix)]
const PREVIEW_COMMAND: &str = "ls -la --color=always {}";

#[cfg(windows)]
const PREVIEW_COMMAND: &str = "dir /Q {}";

impl OnAir for Channel {
    fn find(&mut self, pattern: &str) {
        self.matcher.find(pattern);
    }

    fn results(&mut self, num_entries: u32, offset: u32) -> Vec<Entry> {
        self.matcher.tick();
        self.matcher
            .results(num_entries, offset)
            .into_iter()
            .map(|item| {
                let path = item.matched_string;
                Entry::new(
                    path.clone(),
                    PreviewType::Command(PreviewCommand::new(
                        PREVIEW_COMMAND,
                        " ",
                    )),
                )
                .with_name_match_ranges(item.match_indices)
                .with_icon(FileIcon::from(&path))
            })
            .collect()
    }

    fn get_result(&self, index: u32) -> Option<Entry> {
        self.matcher.get_result(index).map(|item| {
            let path = item.matched_string;
            Entry::new(
                path.clone(),
                PreviewType::Command(PreviewCommand::new(
                    PREVIEW_COMMAND,
                    " ",
                )),
            )
            .with_icon(FileIcon::from(&path))
        })
    }

    fn result_count(&self) -> u32 {
        self.matcher.matched_item_count
    }

    fn total_count(&self) -> u32 {
        self.matcher.total_item_count
    }

    fn running(&self) -> bool {
        self.matcher.status.running
    }

    fn shutdown(&self) {
        self.crawl_handle.abort();
    }
}

#[allow(clippy::unused_async)]
async fn load_dirs(paths: Vec<PathBuf>, injector: Injector<String>) {
    if paths.is_empty() {
        return;
    }
    let current_dir = std::env::current_dir().unwrap();
    let mut builder =
        walk_builder(&paths[0], *DEFAULT_NUM_THREADS, None, None);
    paths[1..].iter().for_each(|path| {
        builder.add(path);
    });
    let walker = builder.build_parallel();

    walker.run(|| {
        let injector = injector.clone();
        let current_dir = current_dir.clone();
        Box::new(move |result| {
            if let Ok(entry) = result {
                if entry.file_type().unwrap().is_dir() {
                    let dir_path = &entry
                        .path()
                        .strip_prefix(&current_dir)
                        .unwrap_or(entry.path())
                        .to_string_lossy();
                    if dir_path == "" {
                        return ignore::WalkState::Continue;
                    }
                    let () = injector.push(dir_path.to_string(), |e, cols| {
                        cols[0] = e.clone().into();
                    });
                }
            }
            ignore::WalkState::Continue
        })
    });
}