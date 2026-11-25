// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod api;
mod arabic;
mod audio;
mod cache;
mod data;
mod fuzzy;
mod models;
mod paths;
mod settings;
mod srt;
mod ui;

use anyhow::Result;
use settings::Settings;

pub const BANNER: &str = include_str!("../assets/banner.txt");

#[tokio::main]
async fn main() -> Result<()> {
    let paths = paths::Paths::new()?;
    paths.ensure_dirs()?;
    let settings = Settings::load(&paths.settings_file);
    settings.save(&paths.settings_file)?;

    let client = api::QuranApiClient::new()?;
    let cache = cache::QuranCache::load(&paths.data_file);
    let db_files = cache::DatabaseFiles::new(&paths.cache_dir);

    let data = if cache.missing_surahs().is_empty() && db_files.missing().is_empty() {
        Some(data::DataHandler::load(cache.clone(), &db_files)?)
    } else {
        None
    };

    let app = app::App::new(settings, paths, client, cache, db_files, data);
    app::run(app).await?;
    Ok(())
}