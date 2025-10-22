use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Error};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Metadata describing a favourited station.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteStation {
    pub id: String,
    pub name: String,
    pub provider: String,
}

/// File-backed favourites store.
pub struct FavoritesStore {
    path: PathBuf,
    favorites: Vec<FavoriteStation>,
}

impl FavoritesStore {
    /// Load favourites from disk, falling back to an empty list when the file
    /// does not exist or is corrupted.
    pub fn load() -> Result<Self, Error> {
        let path = favorites_path()?;
        ensure_parent(&path)?;

        let favorites = match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Vec<FavoriteStation>>(&content) {
                Ok(entries) => entries,
                Err(err) => {
                    eprintln!(
                        "warning: favourites file corrupted ({}), starting fresh",
                        err
                    );
                    Vec::new()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => return Err(Error::from(err).context("failed to read favourites file")),
        };

        Ok(Self { path, favorites })
    }

    /// Return a snapshot of all favourite stations.
    pub fn all(&self) -> &[FavoriteStation] {
        &self.favorites
    }

    /// Check whether the provided station is already a favourite.
    pub fn is_favorite(&self, id: &str, provider: &str) -> bool {
        self.favorites
            .iter()
            .any(|fav| fav.id == id && fav.provider == provider)
    }

    /// Add a station to favourites if it is not already present.
    pub fn add(&mut self, favorite: FavoriteStation) -> Result<(), Error> {
        if !self.is_favorite(&favorite.id, &favorite.provider) {
            self.favorites.push(favorite);
            self.save()?;
        }
        Ok(())
    }

    /// Remove a station from favourites.
    pub fn remove(&mut self, id: &str, provider: &str) -> Result<(), Error> {
        let initial_len = self.favorites.len();
        self.favorites
            .retain(|fav| !(fav.id == id && fav.provider == provider));
        if self.favorites.len() != initial_len {
            self.save()?;
        }
        Ok(())
    }

    /// Toggle a station in favourites, returning whether it was added (`true`) or removed (`false`).
    pub fn toggle(&mut self, favorite: FavoriteStation) -> Result<bool, Error> {
        if self.is_favorite(&favorite.id, &favorite.provider) {
            self.remove(&favorite.id, &favorite.provider)?;
            Ok(false)
        } else {
            self.add(favorite)?;
            Ok(true)
        }
    }

    fn save(&self) -> Result<(), Error> {
        let serialized = serde_json::to_string_pretty(&self.favorites)
            .context("failed to serialize favourites list")?;
        fs::write(&self.path, serialized).context("failed to write favourites file")
    }
}

fn favorites_path() -> Result<PathBuf, Error> {
    let dirs = ProjectDirs::from("io", "tunein-cli", "tunein-cli")
        .ok_or_else(|| Error::msg("unable to determine configuration directory"))?;
    Ok(dirs.config_dir().join("favorites.json"))
}

fn ensure_parent(path: &Path) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create favourites directory")?;
    }
    Ok(())
}
