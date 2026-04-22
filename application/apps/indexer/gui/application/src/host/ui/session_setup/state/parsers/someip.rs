use super::FibexFileInfo;
use crate::host::common::someip_stats::{MessageDistribution, SomeipStatistics};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use someip_messages::MessageId;
use std::path::PathBuf;
use stypes::{ObserveOrigin, SomeIpParserSettings};

#[derive(Debug, Clone, Default)]
pub struct SomeIpParserConfig {
    pub source_paths: Option<Vec<PathBuf>>,
    pub fibex_files: Vec<FibexFileInfo>,
    pub someip_statistics: Option<Box<SomeipStatistics>>,
    pub someip_summary: Box<SomeipSummary>,
    pub someip_tables: Box<SomeipTables>,
}

impl SomeIpParserConfig {
    pub fn new(source_paths: Option<Vec<PathBuf>>) -> Self {
        SomeIpParserConfig {
            source_paths,
            ..Self::default()
        }
    }

    pub fn from_observe_options(settings: &SomeIpParserSettings, origin: &ObserveOrigin) -> Self {
        let source_paths = match origin {
            ObserveOrigin::File(_, _, path_buf) => Some(vec![path_buf.to_owned()]),
            ObserveOrigin::Concat(items) => Some(
                items
                    .iter()
                    .map(|(_, _, path)| path.to_owned())
                    .collect_vec(),
            ),
            ObserveOrigin::Stream(..) => None,
        };

        let fibex_files = settings
            .fibex_file_paths
            .as_ref()
            .map(|paths| {
                paths
                    .iter()
                    .map(PathBuf::from)
                    .map(FibexFileInfo::from_path_lossy)
                    .collect_vec()
            })
            .unwrap_or_default();

        Self {
            source_paths,
            fibex_files,
            someip_statistics: None,
            someip_summary: Box::new(SomeipSummary::default()),
            someip_tables: Box::new(SomeipTables::default()),
        }
    }

    pub fn update_summary(&mut self) {
        if let Some(someip_statistics) = &self.someip_statistics {
            *self.someip_summary = SomeipSummary::new(someip_statistics, &self.someip_tables);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SomeipSummary {
    pub total: ServiceSummary,
    pub selected: ServiceSummary,
}

impl SomeipSummary {
    pub fn new(stats: &SomeipStatistics, tables: &SomeipTables) -> Self {
        let serv_messages = collect(&tables.serv_table.selected_ids, &stats.messages);

        let messages = match serv_messages {
            Some(messages) => messages,
            None => MessageDistribution::default(),
        };

        SomeipSummary {
            total: ServiceSummary {
                ids: stats.total.count(),
                count: stats.total.count(),
                messages: stats.total.values(),
            },
            selected: ServiceSummary {
                ids: tables.count(),
                count: tables.count(),
                messages: messages.values(),
            },
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ServiceSummary {
    pub ids: usize,
    pub count: usize,
    pub messages: [usize; 6],
}

#[derive(Debug, Clone, Default)]
pub struct SomeipTables {
    pub serv_table: TableConfig,
}

impl SomeipTables {
    pub fn count(&self) -> usize {
        self.serv_table.selected_ids.len()
    }

    pub fn take_changed(&mut self) -> bool {
        self.serv_table.take_changed()
    }
}

#[derive(Debug, Clone)]
pub struct TableConfig {
    pub selected_ids: FxHashSet<MessageId>,
    pub column_sort: Option<(usize, bool)>,
    pub is_changed: bool,
    pub is_collapsed: bool,
}

impl Default for TableConfig {
    fn default() -> Self {
        TableConfig {
            selected_ids: FxHashSet::default(),
            column_sort: None,
            is_changed: false,
            is_collapsed: true,
        }
    }
}

impl TableConfig {
    pub fn take_changed(&mut self) -> bool {
        if self.is_changed {
            self.is_changed = false;
            return true;
        }

        false
    }
}

fn collect(
    selected_ids: &FxHashSet<MessageId>,
    ids_with_messages: &FxHashMap<MessageId, MessageDistribution>,
) -> Option<MessageDistribution> {
    if selected_ids.is_empty() {
        None
    } else {
        Some(merge(selected_ids, ids_with_messages))
    }
}

fn merge(
    selected_ids: &FxHashSet<MessageId>,
    ids_with_messages: &FxHashMap<MessageId, MessageDistribution>,
) -> MessageDistribution {
    let mut messages = MessageDistribution::default();

    for selected_id in selected_ids {
        if let Some((_, l)) = ids_with_messages.iter().find(|(id, _)| *id == selected_id) {
            messages.merge(l);
        }
    }

    messages
}
