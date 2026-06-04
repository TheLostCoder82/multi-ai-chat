//! Permission Module Benchmarks
//! 
//! Performance benchmarks for permission operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use multi_ai_chat::permission::permission::{Permission, PermissionScope, PermissionStatus};
use multi_ai_chat::permission::storage::PermissionStorage;
use tokio::runtime::Runtime;

fn bench_permission_creation(c: &mut Criterion) {
    c.bench_function("permission_creation", |b| {
        b.iter(|| {
            Permission::new(
                black_box("agent1".to_string()),
                black_box(PermissionScope::DocumentRead),
            )
        })
    });
}

fn bench_permission_grant(c: &mut Criterion) {
    c.bench_function("permission_grant", |b| {
        b.iter(|| {
            let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
            perm.grant(black_box("admin".to_string()), black_box(Some("reason".to_string())))
        })
    });
}

fn bench_permission_state_transitions(c: &mut Criterion) {
    c.bench_function("permission_full_lifecycle", |b| {
        b.iter(|| {
            let mut perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
            perm.grant("admin".to_string(), None).unwrap();
            perm.deny("admin".to_string(), None).unwrap();
            perm.grant("admin".to_string(), None).unwrap();
            perm.revoke("admin".to_string(), None).unwrap();
        })
    });
}

fn bench_storage_store(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("storage_store", |b| {
        b.iter(|| {
            let storage = PermissionStorage::new();
            let perm = Permission::new("agent1".to_string(), PermissionScope::DocumentRead);
            rt.block_on(storage.store(perm)).unwrap();
        })
    });
}

fn bench_storage_large_dataset(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = PermissionStorage::new();
    
    // Pre-populate with 1000 permissions
    for i in 0..1000 {
        let mut perm = Permission::new(
            format!("agent_{}", i % 100),
            PermissionScope::Custom(format!("scope_{}", i)),
        );
        perm.grant("admin".to_string(), None).unwrap();
        rt.block_on(storage.store(perm)).unwrap();
    }
    
    let mut group = c.benchmark_group("storage_operations");
    
    group.bench_function("get_by_agent", |b| {
        b.iter(|| {
            rt.block_on(storage.get_by_agent(black_box("agent_50")))
        })
    });
    
    group.bench_function("has_permission", |b| {
        b.iter(|| {
            rt.block_on(storage.has_permission(
                black_box("agent_50"),
                black_box(&PermissionScope::DocumentRead),
            ))
        })
    });
    
    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_access");
    
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let storage = std::sync::Arc::new(PermissionStorage::new());
                let mut handles = Vec::new();
                
                for i in 0..size {
                    let storage_clone = std::sync::Arc::clone(&storage);
                    let handle = rt.spawn(async move {
                        let perm = Permission::new(
                            format!("agent_{}", i),
                            PermissionScope::Custom(format!("scope_{}", i)),
                        );
                        storage_clone.store(perm).await.unwrap();
                    });
                    handles.push(handle);
                }
                
                rt.block_on(async {
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            })
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_permission_creation,
    bench_permission_grant,
    bench_permission_state_transitions,
    bench_storage_store,
    bench_storage_large_dataset,
    bench_concurrent_access,
);

criterion_main!(benches);
