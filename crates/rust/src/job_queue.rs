#![allow(clippy::await_holding_refcell_ref)]
use boa_engine::{
    job::{GenericJob, Job, JobExecutor, NativeAsyncJob, PromiseJob, TimeoutJob},
    Context, JsResult,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    mem,
    rc::Rc,
};
use tracing::{info, warn};

pub(crate) struct TokioJobQueue {
    promise_jobs: RefCell<VecDeque<PromiseJob>>,
    async_jobs: RefCell<VecDeque<NativeAsyncJob>>,
    generic_jobs: RefCell<VecDeque<GenericJob>>,
    timeout_jobs: RefCell<BTreeMap<boa_engine::context::time::JsInstant, TimeoutJob>>,
}

impl Default for TokioJobQueue {
    fn default() -> Self {
        Self {
            promise_jobs: RefCell::new(VecDeque::new()),
            async_jobs: RefCell::new(VecDeque::new()),
            generic_jobs: RefCell::new(VecDeque::new()),
            timeout_jobs: RefCell::new(BTreeMap::new()),
        }
    }
}

impl TokioJobQueue {
    fn is_idle(&self, context: &Context) -> bool {
        self.promise_jobs.borrow().is_empty()
            && self.async_jobs.borrow().is_empty()
            && self.generic_jobs.borrow().is_empty()
            && self.has_no_timeout_jobs_to_run(context)
    }

    fn clear(&self) {
        self.promise_jobs.borrow_mut().clear();
        self.async_jobs.borrow_mut().clear();
        self.generic_jobs.borrow_mut().clear();
        self.timeout_jobs.borrow_mut().clear();
    }

    fn has_no_timeout_jobs_to_run(&self, context: &Context) -> bool {
        let now = context.clock().now();
        !self
            .timeout_jobs
            .borrow()
            .iter()
            .any(|(time, _)| &now >= time)
    }
}

impl JobExecutor for TokioJobQueue {
    fn enqueue_job(self: Rc<Self>, job: Job, _context: &mut Context) {
        match job {
            Job::PromiseJob(job) => self.promise_jobs.borrow_mut().push_back(job),
            Job::AsyncJob(job) => self.async_jobs.borrow_mut().push_back(job),
            Job::GenericJob(job) => self.generic_jobs.borrow_mut().push_back(job),
            Job::TimeoutJob(job) => {
                let now = _context.clock().now();
                self.timeout_jobs
                    .borrow_mut()
                    .insert(now + job.timeout(), job);
            }
            _ => warn!("Unsupported job type queued"),
        }
    }

    fn run_jobs(self: Rc<Self>, context: &mut Context) -> JsResult<()> {
        loop {
            let job = { self.promise_jobs.borrow_mut().pop_front() };
            let Some(job) = job else {
                break;
            };
            job.call(context)?;
        }

        loop {
            let job = { self.generic_jobs.borrow_mut().pop_front() };
            let Some(job) = job else {
                break;
            };
            job.call(context)?;
        }

        Ok(())
    }

    async fn run_jobs_async(self: Rc<Self>, context: &RefCell<&mut Context>) -> JsResult<()>
    where
        Self: Sized,
    {
        info!("Running jobs async");

        while !self.is_idle(&context.borrow()) {
            loop {
                let job = { self.promise_jobs.borrow_mut().pop_front() };
                let Some(job) = job else {
                    break;
                };

                if let Err(error) = job.call(&mut context.borrow_mut()) {
                    self.clear();
                    warn!("Error occurred while running promise job, clearing job queue");
                    return Err(error);
                }
            }

            loop {
                let job = { self.generic_jobs.borrow_mut().pop_front() };
                let Some(job) = job else {
                    break;
                };

                if let Err(error) = job.call(&mut context.borrow_mut()) {
                    self.clear();
                    warn!("Error occurred while running generic job, clearing job queue");
                    return Err(error);
                }
            }

            let async_job = { self.async_jobs.borrow_mut().pop_front() };
            if let Some(job) = async_job {
                if let Err(error) = job.call(context).await {
                    self.clear();
                    warn!("Error occurred while running async job, clearing job queue");
                    return Err(error);
                }
                continue;
            }

            if self.has_no_timeout_jobs_to_run(&context.borrow()) {
                break;
            }

            {
                let now = context.borrow().clock().now();
                let mut timeout_jobs = self.timeout_jobs.borrow_mut();
                let mut jobs_to_keep = timeout_jobs.split_off(&now);
                jobs_to_keep.retain(|_, job| !job.is_cancelled());
                let jobs_to_run = mem::replace(&mut *timeout_jobs, jobs_to_keep);
                drop(timeout_jobs);

                for job in jobs_to_run.into_values() {
                    if let Err(error) = job.call(&mut context.borrow_mut()) {
                        self.clear();
                        warn!("Error occurred while running timeout job, clearing job queue");
                        return Err(error);
                    }
                }
            }

            context.borrow_mut().clear_kept_objects();
            tokio::task::yield_now().await;
        }

        info!("Finished running jobs async");
        Ok(())
    }
}
