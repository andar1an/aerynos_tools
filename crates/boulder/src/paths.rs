// SPDX-FileCopyrightText: Copyright © 2020-2023 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    io,
    path::{Path, PathBuf},
};

use stone_recipe::Recipe;

use crate::util;

#[derive(Debug, Clone)]
pub struct Id(String);

impl Id {
    pub fn new(recipe: &Recipe) -> Self {
        Self(format!(
            "{}-{}-{}",
            recipe.source.name, recipe.source.version, recipe.source.release
        ))
    }
}

#[derive(Debug)]
pub struct Paths {
    id: Id,
    host_root: PathBuf,
    guest_root: PathBuf,
    recipe_dir: PathBuf,
}

impl Paths {
    pub async fn new(
        id: Id,
        recipe_path: &Path,
        host_root: impl Into<PathBuf>,
        guest_root: impl Into<PathBuf>,
    ) -> io::Result<Self> {
        let recipe_dir = recipe_path
            .parent()
            .unwrap_or(&PathBuf::default())
            .canonicalize()?;

        let job = Self {
            id,
            host_root: host_root.into().canonicalize()?,
            guest_root: guest_root.into(),
            recipe_dir,
        };

        util::ensure_dir_exists(&job.rootfs().host).await?;
        util::ensure_dir_exists(&job.artefacts().host).await?;
        util::ensure_dir_exists(&job.build().host).await?;
        util::ensure_dir_exists(&job.ccache().host).await?;
        util::ensure_dir_exists(&job.upstreams().host).await?;

        Ok(job)
    }

    pub fn rootfs(&self) -> Mapping {
        Mapping {
            host: self.host_root.join("root").join(&self.id.0),
            guest: "/".into(),
        }
    }

    pub fn artefacts(&self) -> Mapping {
        Mapping {
            host: self.host_root.join("artefacts").join(&self.id.0),
            guest: self.guest_root.join("artefacts"),
        }
    }

    pub fn build(&self) -> Mapping {
        Mapping {
            host: self.host_root.join("build").join(&self.id.0),
            guest: self.guest_root.join("build"),
        }
    }

    pub fn ccache(&self) -> Mapping {
        Mapping {
            host: self.host_root.join("ccache"),
            guest: self.guest_root.join("ccache"),
        }
    }

    pub fn upstreams(&self) -> Mapping {
        Mapping {
            host: self.host_root.join("upstreams"),
            guest: self.guest_root.join("sourcedir"),
        }
    }

    pub fn recipe(&self) -> Mapping {
        Mapping {
            host: self.recipe_dir.clone(),
            guest: self.guest_root.join("recipe"),
        }
    }

    pub fn install(&self) -> Mapping {
        Mapping {
            // TODO: Shitty impossible state, this folder
            // doesn't exist on host
            host: "".into(),
            guest: self.guest_root.join("install"),
        }
    }

    /// For the provided [`Mapping`], return the guest
    /// path as it lives on the host fs
    ///
    /// Example:
    /// - host = "/var/cache/boulder/root/test"
    /// - guest = "/mason/build"
    /// - guest_host_path = "/var/cache/boulder/root/test/mason/build"
    pub fn guest_host_path(&self, mapping: &Mapping) -> PathBuf {
        let relative = mapping.guest.strip_prefix("/").unwrap_or(&mapping.guest);

        self.rootfs().host.join(relative)
    }
}

pub struct Mapping {
    pub host: PathBuf,
    pub guest: PathBuf,
}
