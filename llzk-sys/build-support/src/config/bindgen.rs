use std::path::Path;

use anyhow::Result;
use bindgen::{Bindings, Builder};

pub trait BindgenConfig {
    fn apply(&self, bindgen: Builder) -> Result<Builder>;

    fn include_path(&self, bindgen: Builder, path: &Path) -> Builder {
        bindgen.clang_arg(format!("-I{}", path.join("include").display()))
    }

    fn include_paths(&self, bindgen: Builder, paths: &[&Path]) -> Builder {
        bindgen.clang_args(
            paths
                .iter()
                .map(|path| format!("-I{}", path.join("include").display())),
        )
    }

    fn generate(&self) -> Result<Bindings> {
        Ok(self.apply(Builder::default())?.generate()?)
    }
}

impl BindgenConfig for [&dyn BindgenConfig] {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        self.iter()
            .try_fold(bindgen, |bindgen, conf| conf.apply(bindgen))
    }
}

impl<T1: BindgenConfig, T2: BindgenConfig, T3: BindgenConfig> BindgenConfig for (T1, T2, T3) {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        self.2.apply(self.1.apply(self.0.apply(bindgen)?)?)
    }
}
