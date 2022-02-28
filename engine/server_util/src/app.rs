use actix_web::web::ServiceConfig;
pub use include_dir::include_dir;
use include_dir::Dir;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

static GAME_DIR: Dir<'static> = include_dir!("../../js/public");

/// Registers services that serve static files. Static files are served from the filesystem in debug
/// mode and embedded in the binary in release mode.
///
/// Game client files are assumed to be located in: ../js/public/ relative to the game server.
pub fn static_files() -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        // Allows changing without recompilation.
        #[cfg(debug_assertions)]
        {
            use actix_files as fs;
            cfg.service(fs::Files::new("/admin", "../engine/js/public/").index_file("index.html"))
                .service(fs::Files::new("/", "../js/public/").index_file("index.html"));
        }

        // Allows single-binary distribution.
        #[cfg(not(debug_assertions))]
        {
            use actix_plus_static_files::{build_hashmap_from_included_dir, ResourceFiles};
            cfg.service(ResourceFiles::new(
                "/admin",
                build_hashmap_from_included_dir(&include_dir!("../js/public/")),
            ))
            .service(ResourceFiles::new(
                "/*",
                build_hashmap_from_included_dir(&GAME_DIR),
            ));
        }
    }
}

/// Gets a hash of all game client files as they were at compile time.
pub fn game_static_files_hash() -> u64 {
    fn hash_dir(dir: &Dir) -> u64 {
        let mut hash = 0u64;

        for file in dir.files() {
            let mut hasher = DefaultHasher::new();
            file.path().hash(&mut hasher);
            file.contents().hash(&mut hasher);
            //println!("{:?} -> {}", file.path(), hasher.finish());
            // Order-independent.
            hash ^= hasher.finish();
        }

        for sub_dir in dir.dirs() {
            hash ^= hash_dir(sub_dir);
        }

        hash
    }

    hash_dir(&GAME_DIR)
}
