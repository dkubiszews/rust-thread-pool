// MIT License
//
// Copyright (c) 2022 Dawid Kubiszewski (dawidkubiszewski@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crossbeam_channel::{unbounded, Sender};
use std::thread;

type BoxedClosure = Box<dyn FnOnce() + Send + 'static>;

/// Thread pool.
/// # Example
/// ```
/// use thread_pool::ThreadPool;
/// let mut pool = ThreadPool::new(4);
///
/// pool.schedule(|| {
///   println!("execute task 1");
/// });
/// pool.schedule(|| {
///   println!("execute task 2");
/// });
///
/// pool.flush();
/// ```
pub struct ThreadPool {
    comm_sender: Sender<BoxedClosure>,
    threads_handles: Vec<thread::JoinHandle<()>>,
}

impl ThreadPool {
    /// Creates new a ThreadPool.
    /// # Arguments
    ///
    /// * `pool_size` Number of threads to be started in the pool.
    ///
    /// # Notice
    ///
    /// * Threads are started just after creation of object.
    pub fn new(pool_size: usize) -> Self {
        let mut threads_handles = Vec::new();
        let (comm_sender, comm_receiver) = unbounded::<BoxedClosure>();
        for _ in 0..pool_size {
            let comm_receiver_clone = comm_receiver.clone();
            threads_handles.push(thread::spawn(move || {
                let mut exit = false;
                while !exit {
                    match comm_receiver_clone.recv() {
                        Ok(func) => func(),
                        Err(_) => exit = true,
                    }
                }
            }));
        }
        Self {
            threads_handles: threads_handles,
            comm_sender,
        }
    }

    /// Schedule closure to be executed on the thread pool.
    ///
    /// # Arguements
    ///
    /// * `closure` - closure to be executed on the thread pool.
    pub fn schedule<F>(&mut self, closure: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.comm_sender.send(Box::new(closure)).unwrap()
    }

    /// Blocks until all tasks are executed on the thread poll.
    ///
    /// # Note
    ///
    /// After this method is called:
    /// * no other task can be scheduled,
    /// * all threads are stopped.
    pub fn flush(self) {
        drop(self.comm_sender);
        for thread_handle in self.threads_handles {
            thread_handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time;
    use std::{
        collections::{HashMap, HashSet},
        thread::{self, ThreadId},
    };

    use crate::ThreadPool;
    use crossbeam_channel::unbounded;
    #[test]
    fn pool_with_one_thread() {
        let pool_size: usize = 1;
        let test_strings = vec!["closure_1", "closure_2", "closure_3"];
        let (test_sender, test_receiver) = unbounded::<String>();

        let mut pool = ThreadPool::new(pool_size);

        for test_string in &test_strings {
            let test_sender_clone = test_sender.clone();
            let test_string_clone = test_string.clone();
            pool.schedule(move || {
                test_sender_clone
                    .send(test_string_clone.to_string())
                    .unwrap();
            });
        }

        let mut received_strings = HashSet::new();
        for _ in 0..test_strings.len() {
            // TODO: wait with timeout
            received_strings.insert(test_receiver.recv().unwrap());
        }

        for test_string in test_strings {
            assert!(received_strings.contains(test_string));
        }

        pool.flush();
    }

    #[test]
    fn pool_with_multiple_threads() {
        let pool_size: usize = 4;
        let test_tasks_size: usize = 100;
        let expected_task_index_sum = test_tasks_size * (test_tasks_size + 1) / 2;

        let mut pool = ThreadPool::new(pool_size);
        let (test_sender, test_receiver) = unbounded::<(ThreadId, usize)>();
        for i in 1..=test_tasks_size {
            let test_sender_clone = test_sender.clone();
            pool.schedule(move || {
                test_sender_clone.send((thread::current().id(), i)).unwrap();
                thread::sleep(time::Duration::from_millis(10));
            });
        }

        let mut thread_index_to_execution_count: HashMap<ThreadId, usize> = HashMap::new();
        let mut task_index_sum: usize = 0;
        for _ in 0..test_tasks_size {
            let (thread_index, task_index) = test_receiver.recv().unwrap();
            if !thread_index_to_execution_count.contains_key(&thread_index) {
                thread_index_to_execution_count.insert(thread_index, 0);
            }
            *thread_index_to_execution_count
                .get_mut(&thread_index)
                .unwrap() += 1;
            task_index_sum += task_index;
        }

        assert_eq!(pool_size, thread_index_to_execution_count.len());
        for (_, execution_count) in thread_index_to_execution_count {
            assert!(execution_count > 0);
        }
        assert_eq!(expected_task_index_sum, task_index_sum);

        pool.flush();
    }

    #[test]
    fn after_flush_all_tasks_are_executed() {
        let pool_size: usize = 4;
        let test_tasks_size: usize = 100;

        let mut pool = ThreadPool::new(pool_size);
        let (test_sender, test_receiver) = unbounded::<usize>();

        for index in 0..test_tasks_size {
            let test_sender_clone = test_sender.clone();
            pool.schedule(move || {
                test_sender_clone.send(index).unwrap();
                thread::sleep(time::Duration::from_millis(10));
            });
        }

        pool.flush();

        assert_eq!(test_tasks_size, test_receiver.len());
    }
}
