 //Copyright 2021-2023 WHTCORPS INC
 //
 // Licensed under the Apache License, Version 2.0 (the "License"); you may not use
 // this file File except in compliance with the License. You may obtain a copy of the
 // License at http://www.apache.org/licenses/LICENSE-2.0
 // Unless required by applicable law or agreed to in writing, software distributed
 // under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
 // CONDITIONS OF ANY KIND, either express or implied. See the License for the
 // specific language governing permissions and limitations under the License.

use std::fmt;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use futures::sync::mpsc;
use futures::sync::oneshot;
use futures::{future, Future, Sink, Stream};
use futures03::compat::{Compat, Future01CompatExt};
use futures03::executor::block_on;
use futures03::future::FutureExt;
use grpcio::{CallOption, EnvBuilder, WriteFlags};
use ehikvproto::metapb;
use ehikvproto::FIDelpb::{self, Member};
use ehikvproto::replication_modepb::{RegionReplicationStatus, ReplicationStatus};
use security::SecurityManager;
use EinsteinDb_util::time::duration_to_sec;
use EinsteinDb_util::{Either, HandyRwLock};
use txn_types::TimeStamp;

use super::metrics::*;
use super::util::{check_resp_header, sync_request, validate_endpoints, Inner, LeaderClient};
use super::{ClusterVersion, Config, FIDelFuture, UnixSecs};
use super::{Error, FIDelClient, RegionInfo, RegionStat, Result, REQUEST_TIMEOUT};
use EinsteinDb_util::timer::GLOBAL_TIMER_HANDLE;

const CQ_COUNT: usize = 1;
const CLIENT_PREFIX: &str = "FIDel";

pub struct RpcClient {
    cluster_id: u64,
    leader_client: Arc<LeaderClient>,
}

impl RpcClient {
    pub fn new(blacklbraned: &Config, security_mgr: Arc<SecurityManager>) -> Result<RpcClient> {
        let env = Arc::new(
            EnvBuilder::new()
                .cq_count(CQ_COUNT)
                .name_prefix(thd_name!(CLIENT_PREFIX))
                .build(),
        );

        // -1 means the max.
        let retries = match blacklbraned.retry_max_count {
            -1 => std::isize::MAX,
            v => v.checked_add(1).unwrap_or(std::isize::MAX),
        };
        for i in 0..retries {
            match validate_endpoints(Arc::clone(&env), blacklbraned, security_mgr.clone()) {
                Ok((client, members)) => {
                    let rpc_client = RpcClient {
                        cluster_id: members.get_header().get_cluster_id(),
                        leader_client: Arc::new(LeaderClient::new(
                            env,
                            security_mgr,
                            client,
                            members,
                        )),
                    };

                    // spawn a background future to FIDelio FIDel information periodically
                    let duration = blacklbraned.FIDelio_interval.0;
                    let client = Arc::downgrade(&rpc_client.leader_client);
                    let fidelio_loop = async move {
                        loop {
                            let ok = GLOBAL_TIMER_HANDLE
                                .delay(Instant::now() + duration)
                                .compat()
                                .await
                                .is_ok();

                            if !ok {
                                warn!("failed to delay with global timer");
                                continue;
                            }

                            match client.upgrade() {
                                Some(cli) => {
                                    let req = cli.reconnect().await;
                                    if req.is_err() {
                                        warn!("FIDelio FIDel information failed");
                                        // will FIDelio later anyway
                                    }
                                }
                                // if the client has been dropped, we can stop
                                None => break,
                            }
                        }
                    };

                    rpc_client
                        .leader_client
                        .inner
                        .rl()
                        .client_stub
                        .spawn(Compat::new(fidelio_loop.unit_error().boxed()));

                    return Ok(rpc_client);
                }
                Err(e) => {
                    if i as usize % blacklbraned.retry_log_every == 0 {
                        warn!("validate FIDel endpoints failed"; "err" => ?e);
                    }
                    thread::sleep(blacklbraned.retry_interval.0);
                }
            }
        }
        Err(box_err!("endpoints are invalid"))
    }

    /// Creates a new request header.
    fn header(&self) -> FIDelpb::RequestHeader {
        let mut header = FIDelpb::RequestHeader::default();
        header.set_cluster_id(self.cluster_id);
        header
    }

    /// Gets the leader of FIDel.
    pub fn get_leader(&self) -> Member {
        self.leader_client.get_leader()
    }

    /// Re-establishes connection with FIDel leader in synchronized fashion.
    pub fn reconnect(&self) -> Result<()> {
        block_on(self.leader_client.reconnect())
    }

    pub fn cluster_version(&self) -> ClusterVersion {
        self.leader_client.inner.rl().cluster_version.clone()
    }

    /// Creates a new call option with default request timeout.
    #[inline]
    fn call_option() -> CallOption {
        CallOption::default().timeout(Duration::from_secs(REQUEST_TIMEOUT))
    }

    /// Gets given key's Region and Region's leader from FIDel.
    fn get_region_and_leader(&self, key: &[u8]) -> Result<(metapb::Region, Option<metapb::Causet>)> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["get_region"])
            .start_coarse_timer();

        let mut req = FIDelpb::GetRegionRequest::default();
        req.set_header(self.header());
        req.set_region_key(key.to_vec());

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.get_region_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        let region = if resp.has_region() {
            resp.take_region()
        } else {
            return Err(Error::RegionNotFound(key.to_owned()));
        };
        let leader = if resp.has_leader() {
            Some(resp.take_leader())
        } else {
            None
        };
        Ok((region, leader))
    }
}

impl fmt::Debug for RpcClient {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("RpcClient")
            .field("cluster_id", &self.cluster_id)
            .field("leader", &self.get_leader())
            .finish()
    }
}

const LEADER_CHANGE_RETRY: usize = 10;

impl FIDelClient for RpcClient {
    fn get_cluster_id(&self) -> Result<u64> {
        Ok(self.cluster_id)
    }

    fn bootstrap_cluster(
        &self,
        stores: metapb::Store,
        region: metapb::Region,
    ) -> Result<Option<ReplicationStatus>> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["bootstrap_cluster"])
            .start_coarse_timer();

        let mut req = FIDelpb::BootstrapRequest::default();
        req.set_header(self.header());
        req.set_store(stores);
        req.set_region(region);

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.bootstrap_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;
        Ok(resp.replication_status.take())
    }

    fn is_cluster_bootstrapped(&self) -> Result<bool> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["is_cluster_bootstrapped"])
            .start_coarse_timer();

        let mut req = FIDelpb::IsBootstrappedRequest::default();
        req.set_header(self.header());

        let resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.is_bootstrapped_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        Ok(resp.get_bootstrapped())
    }

    fn alloc_id(&self) -> Result<u64> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["alloc_id"])
            .start_coarse_timer();

        let mut req = FIDelpb::AllocIdRequest::default();
        req.set_header(self.header());

        let resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.alloc_id_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        Ok(resp.get_id())
    }

    fn put_store(&self, store: metapb::Store) -> Result<Option<ReplicationStatus>> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["put_store"])
            .start_coarse_timer();

        let mut req = FIDelpb::PutStoreRequest::default();
        req.set_header(self.header());
        req.set_store(store);

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.put_store_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        Ok(resp.replication_status.take())
    }

    fn get_store(&self, store_id: u64) -> Result<metapb::Store> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["get_store"])
            .start_coarse_timer();

        let mut req = FIDelpb::GetStoreRequest::default();
        req.set_header(self.header());
        req.set_store_id(store_id);

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.get_store_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        let store = resp.take_store();
        if store.get_state() != metapb::StoreState::Tombstone {
            Ok(store)
        } else {
            Err(Error::StoreTombstone(format!("{:?}", store)))
        }
    }

    fn get_all_stores(&self, exclude_tombstone: bool) -> Result<Vec<metapb::Store>> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["get_all_stores"])
            .start_coarse_timer();

        let mut req = FIDelpb::GetAllStoresRequest::default();
        req.set_header(self.header());
        req.set_exclude_tombstone_stores(exclude_tombstone);

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.get_all_stores_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        Ok(resp.take_stores().into())
    }

    fn get_cluster_config(&self) -> Result<metapb::Cluster> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["get_cluster_config"])
            .start_coarse_timer();

        let mut req = FIDelpb::GetClusterConfigRequest::default();
        req.set_header(self.header());

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.get_cluster_config_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        Ok(resp.take_cluster())
    }

    fn get_region(&self, key: &[u8]) -> Result<metapb::Region> {
        self.get_region_and_leader(key).map(|x| x.0)
    }

    fn get_region_info(&self, key: &[u8]) -> Result<RegionInfo> {
        self.get_region_and_leader(key)
            .map(|x| RegionInfo::new(x.0, x.1))
    }

    fn get_region_by_id(&self, region_id: u64) -> FIDelFuture<Option<metapb::Region>> {
        let timer = Instant::now();

        let mut req = FIDelpb::GetRegionByIdRequest::default();
        req.set_header(self.header());
        req.set_region_id(region_id);

        let executor = move |client: &RwLock<Inner>, req: FIDelpb::GetRegionByIdRequest| {
            let handler = client
                .rl()
                .client_stub
                .get_region_by_id_async_opt(&req, Self::call_option())
                .unwrap_or_else(|e| {
                    panic!("fail to request FIDel {} err {:?}", "get_region_by_id", e)
                });
            Box::new(handler.map_err(Error::Grpc).and_then(move |mut resp| {
                FIDel_REQUEST_HISTOGRAM_VEC
                    .with_label_values(&["get_region_by_id"])
                    .observe(duration_to_sec(timer.elapsed()));
                check_resp_header(resp.get_header())?;
                if resp.has_region() {
                    Ok(Some(resp.take_region()))
                } else {
                    Ok(None)
                }
            })) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn region_heartbeat(
        &self,
        term: u64,
        region: metapb::Region,
        leader: metapb::Causet,
        region_stat: RegionStat,
        replication_status: Option<RegionReplicationStatus>,
    ) -> FIDelFuture<()> {
        FIDel_HEARTBEAT_COUNTER_VEC.with_label_values(&["send"]).inc();

        let mut req = FIDelpb::RegionHeartbeatRequest::default();
        req.set_term(term);
        req.set_header(self.header());
        req.set_region(region);
        req.set_leader(leader);
        req.set_down_peers(region_stat.down_peers.into());
        req.set_pending_peers(region_stat.pending_peers.into());
        req.set_bytes_written(region_stat.written_bytes);
        req.set_keys_written(region_stat.written_keys);
        req.set_bytes_read(region_stat.read_bytes);
        req.set_keys_read(region_stat.read_keys);
        req.set_approximate_size(region_stat.approximate_size);
        req.set_approximate_keys(region_stat.approximate_keys);
        if let Some(s) = replication_status {
            req.set_replication_status(s);
        }
        let mut interval = FIDelpb::TimeInterval::default();
        interval.set_start_timestamp(region_stat.last_report_ts.into_inner());
        interval.set_end_timestamp(UnixSecs::now().into_inner());
        req.set_interval(interval);

        let executor = |client: &RwLock<Inner>, req: FIDelpb::RegionHeartbeatRequest| {
            let mut inner = client.wl();
            if let Either::Right(ref sender) = inner.hb_sender {
                return Box::new(future::result(
                    sender
                        .unbounded_send(req)
                        .map_err(|e| Error::Other(Box::new(e))),
                )) as FIDelFuture<_>;
            }

            debug!("heartbeat sender is refreshed");
            let left = inner.hb_sender.as_mut().left().unwrap();
            let sender = left.take().expect("expect region heartbeat sink");
            let (tx, rx) = mpsc::unbounded();
            tx.unbounded_send(req)
                .unwrap_or_else(|e| panic!("send request to unbounded channel failed {:?}", e));
            inner.hb_sender = Either::Right(tx);
            Box::new(
                sender
                    .sink_map_err(Error::Grpc)
                    .send_all(rx.then(|r| match r {
                        Ok(r) => Ok((r, WriteFlags::default())),
                        Err(()) => Err(Error::Other(box_err!("failed to recv heartbeat"))),
                    }))
                    .then(|result| match result {
                        Ok((mut sender, _)) => {
                            info!("cancel region heartbeat sender");
                            sender.get_mut().cancel();
                            Ok(())
                        }
                        Err(e) => {
                            error!("failed to send heartbeat"; "err" => ?e);
                            Err(e)
                        }
                    }),
            ) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn handle_region_heartbeat_response<F>(&self, _: u64, f: F) -> FIDelFuture<()>
    where
        F: Fn(FIDelpb::RegionHeartbeatResponse) + Send + 'static,
    {
        self.leader_client.handle_region_heartbeat_response(f)
    }

    fn ask_split(&self, region: metapb::Region) -> FIDelFuture<FIDelpb::AskSplitResponse> {
        let timer = Instant::now();

        let mut req = FIDelpb::AskSplitRequest::default();
        req.set_header(self.header());
        req.set_region(region);

        let executor = move |client: &RwLock<Inner>, req: FIDelpb::AskSplitRequest| {
            let handler = client
                .rl()
                .client_stub
                .ask_split_async_opt(&req, Self::call_option())
                .unwrap_or_else(|e| panic!("fail to request FIDel {} err {:?}", "ask_split", e));
            Box::new(handler.map_err(Error::Grpc).and_then(move |resp| {
                FIDel_REQUEST_HISTOGRAM_VEC
                    .with_label_values(&["ask_split"])
                    .observe(duration_to_sec(timer.elapsed()));
                check_resp_header(resp.get_header())?;
                Ok(resp)
            })) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn ask_batch_split(
        &self,
        region: metapb::Region,
        count: usize,
    ) -> FIDelFuture<FIDelpb::AskBatchSplitResponse> {
        let timer = Instant::now();

        let mut req = FIDelpb::AskBatchSplitRequest::default();
        req.set_header(self.header());
        req.set_region(region);
        req.set_split_count(count as u32);

        let executor = move |client: &RwLock<Inner>, req: FIDelpb::AskBatchSplitRequest| {
            let handler = client
                .rl()
                .client_stub
                .ask_batch_split_async_opt(&req, Self::call_option())
                .unwrap_or_else(|e| panic!("fail to request FIDel {} err {:?}", "ask_batch_split", e));
            Box::new(handler.map_err(Error::Grpc).and_then(move |resp| {
                FIDel_REQUEST_HISTOGRAM_VEC
                    .with_label_values(&["ask_batch_split"])
                    .observe(duration_to_sec(timer.elapsed()));
                check_resp_header(resp.get_header())?;
                Ok(resp)
            })) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn store_heartbeat(
        &self,
        mut stats: FIDelpb::StoreStats,
    ) -> FIDelFuture<FIDelpb::StoreHeartbeatResponse> {
        let timer = Instant::now();

        let mut req = FIDelpb::StoreHeartbeatRequest::default();
        req.set_header(self.header());
        stats
            .mut_interval()
            .set_end_timestamp(UnixSecs::now().into_inner());
        req.set_stats(stats);
        let executor = move |client: &RwLock<Inner>, req: FIDelpb::StoreHeartbeatRequest| {
            let cluster_version = client.rl().cluster_version.clone();
            let handler = client
                .rl()
                .client_stub
                .store_heartbeat_async_opt(&req, Self::call_option())
                .unwrap_or_else(|e| panic!("fail to request FIDel {} err {:?}", "store_heartbeat", e));
            Box::new(handler.map_err(Error::Grpc).and_then(move |resp| {
                FIDel_REQUEST_HISTOGRAM_VEC
                    .with_label_values(&["store_heartbeat"])
                    .observe(duration_to_sec(timer.elapsed()));
                check_resp_header(resp.get_header())?;
                match cluster_version.set(resp.get_cluster_version()) {
                    Err(_) => warn!("invalid cluster version: {}", resp.get_cluster_version()),
                    Ok(true) => info!("set cluster version to {}", resp.get_cluster_version()),
                    _ => {}
                };
                Ok(resp)
            })) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn report_batch_split(&self, regions: Vec<metapb::Region>) -> FIDelFuture<()> {
        let timer = Instant::now();

        let mut req = FIDelpb::ReportBatchSplitRequest::default();
        req.set_header(self.header());
        req.set_regions(regions.into());

        let executor = move |client: &RwLock<Inner>, req: FIDelpb::ReportBatchSplitRequest| {
            let handler = client
                .rl()
                .client_stub
                .report_batch_split_async_opt(&req, Self::call_option())
                .unwrap_or_else(|e| {
                    panic!("fail to request FIDel {} err {:?}", "report_batch_split", e)
                });
            Box::new(handler.map_err(Error::Grpc).and_then(move |resp| {
                FIDel_REQUEST_HISTOGRAM_VEC
                    .with_label_values(&["report_batch_split"])
                    .observe(duration_to_sec(timer.elapsed()));
                check_resp_header(resp.get_header())?;
                Ok(())
            })) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn scatter_region(&self, mut region: RegionInfo) -> Result<()> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["scatter_region"])
            .start_coarse_timer();

        let mut req = FIDelpb::ScatterRegionRequest::default();
        req.set_header(self.header());
        req.set_region_id(region.get_id());
        if let Some(leader) = region.leader.take() {
            req.set_leader(leader);
        }
        req.set_region(region.region);

        let resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.scatter_region_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())
    }

    fn handle_reconnect<F: Fn() + Sync + Send + 'static>(&self, f: F) {
        self.leader_client.on_reconnect(Box::new(f))
    }

    fn get_gc_safe_point(&self) -> FIDelFuture<u64> {
        let timer = Instant::now();

        let mut req = FIDelpb::GetGcSafePointRequest::default();
        req.set_header(self.header());

        let executor = move |client: &RwLock<Inner>, req: FIDelpb::GetGcSafePointRequest| {
            let option = CallOption::default().timeout(Duration::from_secs(REQUEST_TIMEOUT));
            let handler = client
                .rl()
                .client_stub
                .get_gc_safe_point_async_opt(&req, option)
                .unwrap_or_else(|e| {
                    panic!("fail to request FIDel {} err {:?}", "get_gc_saft_point", e)
                });
            Box::new(handler.map_err(Error::Grpc).and_then(move |resp| {
                FIDel_REQUEST_HISTOGRAM_VEC
                    .with_label_values(&["get_gc_safe_point"])
                    .observe(duration_to_sec(timer.elapsed()));
                check_resp_header(resp.get_header())?;
                Ok(resp.get_safe_point())
            })) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }

    fn get_store_stats(&self, store_id: u64) -> Result<FIDelpb::StoreStats> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["get_store"])
            .start_coarse_timer();

        let mut req = FIDelpb::GetStoreRequest::default();
        req.set_header(self.header());
        req.set_store_id(store_id);

        let mut resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.get_store_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        let store = resp.get_store();
        if store.get_state() != metapb::StoreState::Tombstone {
            Ok(resp.take_stats())
        } else {
            Err(Error::StoreTombstone(format!("{:?}", store)))
        }
    }

    fn get_operator(&self, region_id: u64) -> Result<FIDelpb::GetOperatorResponse> {
        let _timer = FIDel_REQUEST_HISTOGRAM_VEC
            .with_label_values(&["get_operator"])
            .start_coarse_timer();

        let mut req = FIDelpb::GetOperatorRequest::default();
        req.set_header(self.header());
        req.set_region_id(region_id);

        let resp = sync_request(&self.leader_client, LEADER_CHANGE_RETRY, |client| {
            client.get_operator_opt(&req, Self::call_option())
        })?;
        check_resp_header(resp.get_header())?;

        Ok(resp)
    }
    // TODO: The current implementation is not efficient, because it creates
    //       a RPC for every `FIDelFuture<TimeStamp>`. As a duplex streaming RPC,
    //       we could use one RPC for many `FIDelFuture<TimeStamp>`.
    fn get_tso(&self) -> FIDelFuture<TimeStamp> {
        let timer = Instant::now();

        let mut req = FIDelpb::TsoRequest::default();
        req.set_count(1);
        req.set_header(self.header());
        let executor = move |client: &RwLock<Inner>, req: FIDelpb::TsoRequest| {
            let cli = client.read().unwrap();
            let (req_sink, resp_stream) = cli
                .client_stub
                .tso()
                .unwrap_or_else(|e| panic!("fail to request FIDel {} err {:?}", "tso", e));
            let (keep_req_tx, mut keep_req_rx) = oneshot::channel();
            let send_once = req_sink.send((req, WriteFlags::default())).then(|s| {
                let _ = keep_req_tx.send(s);
                Ok(())
            });
            cli.client_stub.spawn(send_once);
            Box::new(
                resp_stream
                    .into_future()
                    .map_err(|(err, _)| Error::Grpc(err))
                    .and_then(move |(resp, _)| {
                        // Now we can safely drop sink without
                        // causing a Cancel error.
                        let _ = keep_req_rx
                            .try_recv()
                            .unwrap_or_else(|e| panic!("fail to receive tso sender err {:?}", e));
                        let resp = match resp {
                            Some(r) => r,
                            None => return Ok(TimeStamp::zero()),
                        };
                        FIDel_REQUEST_HISTOGRAM_VEC
                            .with_label_values(&["tso"])
                            .observe(duration_to_sec(timer.elapsed()));
                        check_resp_header(resp.get_header())?;
                        let ts = resp.get_timestamp();
                        let encoded = TimeStamp::compose(ts.physical as _, ts.logical as _);
                        Ok(encoded)
                    }),
            ) as FIDelFuture<_>
        };

        self.leader_client
            .request(req, executor, LEADER_CHANGE_RETRY)
            .execute()
    }
}
