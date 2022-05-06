use crate::{
    events::{CallbackEvent, NativeError, NativeErrorKind, SearchOperationResult},
    operations::{OperationAPI, OperationResult},
    state::SessionStateAPI,
};
use indexer_base::progress::Severity;
use log::debug;
use processor::search::{FilterStats, SearchFilter, SearchHolder, SearchResults};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task,
    time::{timeout, Duration},
};

const TRACKING_INTERVAL_MS: u64 = 250;

pub async fn handle(
    operation_api: &OperationAPI,
    filters: Vec<SearchFilter>,
    state: SessionStateAPI,
) -> OperationResult<SearchOperationResult> {
    debug!("RUST: Search operation is requested");
    state.drop_search().await?;
    if filters.is_empty() {
        debug!("RUST: Search will be dropped. Filters are empty");
        operation_api.emit(CallbackEvent::SearchUpdated(0));
        Ok(Some(SearchOperationResult {
            found: 0,
            stats: FilterStats::new(vec![]),
        }))
    } else {
        let mut search_holder = state.get_search_holder().await?;
        search_holder.set_filters(&mut filters.iter());
        let search_res_file = search_holder.out_file_path.clone();
        let (tx_result, mut rx_result): (
            Sender<(SearchHolder, SearchResults)>,
            Receiver<(SearchHolder, SearchResults)>,
        ) = channel(1);
        task::spawn(async move {
            let search_results = search_holder.execute_search();
            if tx_result
                .send((search_holder, search_results))
                .await
                .is_ok()
            {}
        });
        let search_results = loop {
            match timeout(
                Duration::from_millis(TRACKING_INTERVAL_MS as u64),
                rx_result.recv(),
            )
            .await
            {
                Ok(recv_results) => {
                    break match recv_results {
                        Some((search_holder, search_results)) => match search_results {
                            Ok((file_path, matches, stats)) => {
                                Ok((file_path, matches.len(), matches, stats, search_holder))
                            }
                            Err(err) => Err(NativeError {
                                severity: Severity::ERROR,
                                kind: NativeErrorKind::OperationSearch,
                                message: Some(format!("Fail to execute search. Error: {}", err)),
                            }),
                        },
                        None => Err(NativeError {
                            severity: Severity::ERROR,
                            kind: NativeErrorKind::OperationSearch,
                            message: Some("Fail to receive search results".to_string()),
                        }),
                    };
                }
                Err(_) => {
                    match state.update_search_result(search_res_file.clone()).await {
                        Ok(found) => {
                            operation_api.emit(CallbackEvent::SearchUpdated(found as u64));
                        }
                        Err(err) => {
                            break Err(err);
                        }
                    }
                    if let Err(err) = state.update_search_result(search_res_file.clone()).await {
                        break Err(err);
                    }
                }
            };
        };
        match search_results {
            Ok((file_path, found, matches, stats, search_holder)) => {
                state.set_search_holder(search_holder).await?;
                state.set_matches(Some(matches)).await?;
                if found == 0 {
                    operation_api.emit(CallbackEvent::SearchUpdated(0));
                    Ok(Some(SearchOperationResult { found, stats }))
                } else {
                    state.update_search_result(file_path.clone()).await?;
                    operation_api.emit(CallbackEvent::SearchUpdated(found as u64));
                    Ok(Some(SearchOperationResult { found, stats }))
                }
            }
            Err(err) => Err(err),
        }
    }
}