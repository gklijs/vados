//! Bounded scoped workers with diagnostics replayed in input order.
use std::cell::RefCell;
use std::sync::Mutex;

thread_local! {
    static MESSAGES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

pub(crate) fn message(text: String) {
    MESSAGES.with(|messages| match &mut *messages.borrow_mut() {
        Some(messages) => messages.push(text),
        None => println!("{text}"),
    });
}

fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
    struct Restore(Option<Vec<String>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            MESSAGES.with(|messages| *messages.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(MESSAGES.with(|messages| messages.replace(Some(vec![]))));
    let result = f();
    let messages = MESSAGES.with(|messages| messages.borrow_mut().take().unwrap());
    (result, messages)
}

pub(crate) fn map<T: Send, R: Send>(
    tasks: Vec<T>,
    jobs: usize,
    f: impl Fn(T) -> R + Sync,
) -> Vec<R> {
    let workers = jobs.max(1).min(tasks.len());
    if workers <= 1 {
        return tasks.into_iter().map(f).collect();
    }
    let queue = Mutex::new(tasks.into_iter().enumerate());
    let mut completed = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let queue = &queue;
                let f = &f;
                scope.spawn(move || {
                    let mut results = vec![];
                    loop {
                        let task = queue.lock().unwrap().next();
                        let Some((index, task)) = task else { break };
                        results.push((index, capture(|| f(task))));
                    }
                    results
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    completed.sort_unstable_by_key(|(index, _)| *index);
    completed
        .into_iter()
        .map(|(_, (result, messages))| {
            for text in messages {
                message(text);
            }
            result
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn results_and_diagnostics_keep_input_order() {
        let (results, messages) = capture(|| {
            map((0..32).collect(), 4, |i| {
                message(format!("task {i}"));
                i * 2
            })
        });
        assert_eq!(results, (0..32).map(|i| i * 2).collect::<Vec<_>>());
        assert_eq!(
            messages,
            (0..32).map(|i| format!("task {i}")).collect::<Vec<_>>()
        );
    }
}
