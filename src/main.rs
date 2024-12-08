use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::sync::{
    mpsc::{self, Sender}, Barrier, Mutex
};
use tracing::info;

mod log_utils;

#[derive(Clone)]
struct Task {
    id: &'static str,
    dependencies: Vec<&'static str>, // List of task IDs this task depends on
    dependents: Vec<&'static str>,   // List of task IDs that depend on this task
    process: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    let _guard = log_utils::init_tracing("logs").await.unwrap();

    // define tasks
    let tasks: Arc<Mutex<HashMap<&str, Task>>> = Arc::new(Mutex::new(HashMap::new()));

    // channels for notifying dependents
    let channels: Arc<Mutex<HashMap<&str, Sender<()>>>> = Arc::new(Mutex::new(HashMap::new()));

    // task process definitions
    let task_definitions = vec![
        ("A", vec![], vec!["B", "C"]),
        ("B", vec!["A"], vec!["D"]),
        ("C", vec!["A"], vec!["D"]),
        ("D", vec!["B", "C"], vec!["E"]),
        ("E", vec!["D"], vec!["F"]),
        ("F", vec!["E"], vec!["G"]),
        ("G", vec!["F"], vec!["H"]),
    ];

    let barrier = Arc::new(Barrier::new(task_definitions.len() + 1));

    // create tasks
    for (id, dependencies, dependents) in task_definitions {
        let (tx, rx) = mpsc::channel(1);
        
        {
            channels.lock().await.insert(id, tx);
        }

        let task = Task {
            id,
            dependencies,
            dependents,
            process: Arc::new(move || {
                Box::pin(async move {
                    info!("Task {} started", id);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                })
            }),
        };

        {
            tasks.lock().await.insert(id, task.clone());
        }

        let channels = channels.clone();
        let barrier = Arc::clone(&barrier);

        tokio::spawn(async move {
            if !task.dependencies.is_empty() {
                wait_for_dependencies(&task.id, rx).await;
            }

            (task.process)().await;
            notify_dependents(task.id, &task.dependents, channels.clone()).await;

            barrier.wait().await;
        });
    }

    barrier.wait().await;
    info!("All tasks completed.");
}

/// Notify all dependents of a task
async fn notify_dependents(
    task_id: &str,
    dependents: &[&str],
    channels: Arc<Mutex<HashMap<&str, Sender<()>>>>,
) {
    info!("Task {} completed. Notifying dependents", task_id);

    for &dependent in dependents {
        {
            let mut channels = channels.lock().await;
            if !channels.contains_key(dependent) {
                continue;
            } else {
                info!("Notifying Task {}", dependent);
                let _ = channels.get(dependent).unwrap().send(()).await;
                channels.remove(dependent);
            }
        }
    }
}

/// Wait for all dependencies of a task to complete.
async fn wait_for_dependencies(task_id: &str, mut rx: mpsc::Receiver<()>) {
    info!("Waiting for dependencies of Task {}...", task_id);
    while rx.recv().await.is_some() {
        info!("Dependency completed for Task {}", task_id);
    }
}
