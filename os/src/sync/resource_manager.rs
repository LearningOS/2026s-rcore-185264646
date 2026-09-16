//! Resource Manager
// Algorithm: see https://learningos.cn/rCore-Tutorial-Guide/chapter8/5exercise.html

const MAX_RESOURCE_COUNT: usize = 16;
pub const MAX_THREAD_ID: usize = 32;

/// ResourceManager
pub struct ResourceManager {
    avail: [u32; MAX_RESOURCE_COUNT],
    alloc: [[u32; MAX_RESOURCE_COUNT]; MAX_THREAD_ID],
    need: [[u32; MAX_RESOURCE_COUNT]; MAX_THREAD_ID],
}

impl ResourceManager {
    /// create an empty resource manager
    pub fn new() -> Self {
        Self {
            avail: [0; MAX_RESOURCE_COUNT],
            alloc: [[0; MAX_RESOURCE_COUNT]; MAX_THREAD_ID],
            need: [[0; MAX_RESOURCE_COUNT]; MAX_THREAD_ID],
        }
    }

    /// create a resoource `id` with available count `avail`
    pub fn create_res(&mut self, id: usize, avail: u32) {
        assert!(id < MAX_RESOURCE_COUNT);
        self.avail[id] = avail;
    }

    /// destroy a resource `ix`
    pub fn destroy_res(&mut self, id: u32) {
        let id = id as usize;
        assert!(id < MAX_RESOURCE_COUNT);
        self.avail[id] = 0;
    }

    fn alloc_res_unchecked(&mut self, tid: usize, id: usize) {
        self.need[tid][id] += 1;
    }

    /// release a resource
    pub fn free_res(&mut self, tid: usize, id: usize) {
        self.alloc[tid][id] -= 1;
        self.avail[id] += 1;
    }

    /// status rollback when alloc failed
    fn alloc_res_rollback(&mut self, tid: usize, id: usize) {
        self.need[tid][id] -= 1;
    }

    fn check_res(&self) -> bool {
        let mut finish = [false; MAX_THREAD_ID];
        let mut work = self.avail.clone();
        for _ in 0..MAX_THREAD_ID {
            'out: for (i, finished) in finish.iter_mut().enumerate() {
                if *finished {
                    continue;
                }

                for (j, need_res) in self.need[i].iter().enumerate() {
                    if work[j] < *need_res {
                        continue 'out;
                    }
                }

                for (j, alloc_res) in self.alloc[i].iter().enumerate() {
                    work[j] += *alloc_res;
                }

                *finished = true;
            }
        }

        finish.iter().all(|i| *i)
    }

    /// allocate a resource
    pub fn alloc_res(&mut self, tid: usize, id: usize) -> bool {
        assert!(id < MAX_RESOURCE_COUNT);
        assert!(tid < MAX_THREAD_ID);
        self.alloc_res_unchecked(tid, id);
        if !self.check_res() {
            self.alloc_res_rollback(tid, id);
            return false;
        }

        true
    }

    /// allocate a resource done
    pub fn alloc_res_done(&mut self, tid: usize, id: usize) {
        assert!(id < MAX_RESOURCE_COUNT);
        assert!(tid < MAX_THREAD_ID);
        assert!(self.need[tid][id] > 0);
        self.need[tid][id] -= 1;
        self.avail[id] -= 1;
        self.alloc[tid][id] += 1;
    }
}
