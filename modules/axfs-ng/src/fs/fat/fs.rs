use core::marker::PhantomPinned;

use alloc::sync::Arc;
use axdriver::AxBlockDevice;
use axfs_ng_vfs::{
    DirEntry, Filesystem, FilesystemOps, Reference, StatFs, VfsResult, path::MAX_NAME_LEN,
};
use lock_api::{Mutex, MutexGuard, RawMutex};
use slab::Slab;

use crate::disk::SeekableDisk;

use super::{dir::FatDirNode, ff, util::into_vfs_err};

pub struct FatFilesystemInner {
    pub inner: ff::FileSystem,
    inode_allocator: Slab<()>,
    _pinned: PhantomPinned,
}
impl FatFilesystemInner {
    pub(crate) fn alloc_inode(&mut self) -> u64 {
        self.inode_allocator.insert(()) as u64 + 1
    }
    pub(crate) fn release_inode(&mut self, ino: u64) {
        self.inode_allocator.remove(ino as usize - 1);
    }
}

pub struct FatFilesystem<M> {
    inner: Mutex<M, FatFilesystemInner>,
    root_dir: Mutex<M, Option<DirEntry<M>>>,
}

impl<M: RawMutex + Send + Sync + 'static> FatFilesystem<M> {
    pub fn new(dev: AxBlockDevice) -> Filesystem<M> {
        let mut inner = FatFilesystemInner {
            inner: ff::FileSystem::new(SeekableDisk::new(dev), fatfs::FsOptions::new())
                .expect("failed to initialize FAT filesystem"),
            inode_allocator: Slab::new(),
            _pinned: PhantomPinned,
        };
        let root_inode = inner.alloc_inode();
        let result = Arc::new(Self {
            inner: Mutex::new(inner),
            root_dir: Mutex::default(),
        });

        let root_dir = DirEntry::new_dir(
            |this| {
                FatDirNode::new(
                    result.clone(),
                    result.lock().inner.root_dir(),
                    root_inode,
                    this,
                )
            },
            Reference::root(),
        );
        *result.root_dir.lock() = Some(root_dir);
        Filesystem::new(result)
    }
}
impl<M: RawMutex> FatFilesystem<M> {
    pub(crate) fn lock(&self) -> MutexGuard<M, FatFilesystemInner> {
        self.inner.lock()
    }
}

impl<M: RawMutex + Send + Sync> FilesystemOps<M> for FatFilesystem<M> {
    fn name(&self) -> &str {
        "vfat"
    }

    fn root_dir(&self) -> DirEntry<M> {
        self.root_dir.lock().clone().unwrap()
    }

    fn stat(&self) -> VfsResult<StatFs> {
        let fs = self.inner.lock();
        let stats = fs.inner.stats().map_err(into_vfs_err)?;
        Ok(StatFs {
            fs_type: 0x65735546, // fuse
            block_size: stats.cluster_size() as _,
            blocks: stats.total_clusters() as _,
            blocks_free: stats.free_clusters() as _,
            blocks_available: stats.free_clusters() as _,

            file_count: 0,
            free_file_count: 0,

            name_length: MAX_NAME_LEN as _,
            fragment_size: 0,
            mount_flags: 0,
        })
    }
}
