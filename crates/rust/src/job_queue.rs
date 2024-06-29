#![allow(clippy::await_holding_refcell_ref)]
use boa_engine::{
    job::{FutureJob, JobQueue, NativeJob},
    Context,
};
use tracing::{info, warn};

pub(crate) struct TokioJobQueue {
    jobs: std::cell::RefCell<std::collections::VecDeque<NativeJob>>,
    futures: std::cell::RefCell<std::collections::VecDeque<FutureJob>>,
}

impl Default for TokioJobQueue {
    fn default() -> Self {
        Self {
            jobs: std::cell::RefCell::new(std::collections::VecDeque::new()),
            futures: std::cell::RefCell::new(std::collections::VecDeque::new()),
        }
    }
}

// https://zenn.dev/itte/articles/5c8e5c191e386b#%E3%82%B8%E3%83%A7%E3%83%96%E3%82%AD%E3%83%A5%E3%83%BC%E3%82%92%E5%AE%9F%E8%A3%85%E3%81%97%E3%81%A6%E3%81%BF%E3%82%8B
impl JobQueue for TokioJobQueue {
    fn enqueue_promise_job(&self, job: NativeJob, _context: &mut Context) {
        self.jobs.borrow_mut().push_back(job);
    }

    fn enqueue_future_job(&self, future: FutureJob, _context: &mut Context) {
        self.futures.borrow_mut().push_back(future);
    }

    fn run_jobs(&self, context: &mut Context) {
        let mut next_job = self.jobs.borrow_mut().pop_front();
        while let Some(job) = next_job {
            if job.call(context).is_err() {
                self.jobs.borrow_mut().clear();
                warn!("Error occurred while running job, clearing job queue");
                return;
            };
            next_job = self.jobs.borrow_mut().pop_front();
        }
    }

    fn run_jobs_async<'a, 'ctx, 'fut>(
        &'a self,
        context: &'ctx mut Context,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'fut>>
    where
        'a: 'fut,
        'ctx: 'fut,
    {
        Box::pin(async {
            let local = tokio::task::LocalSet::new();
            info!("Running jobs async");
            local
                .run_until(async {
                    while !(self.jobs.borrow().is_empty() && self.futures.borrow().is_empty()) {
                        context.run_jobs();

                        if let Some(res) = self.futures.borrow_mut().pop_front() {
                            let handle = res.await;
                            context.enqueue_job(handle)
                        }
                    }
                })
                .await;
            info!("Finished running jobs async");
        })
    }
}
