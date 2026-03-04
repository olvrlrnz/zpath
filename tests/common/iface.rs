use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use zbus::{
    interface,
    zvariant::{OwnedValue, Type, Value},
};
use zpath::{ZPath, ZPathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type, Value, OwnedValue)]
pub struct Volume {
    pub device_path: String,
    pub mount_path: ZPathBuf,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, Type, Value, OwnedValue)]
pub struct Volumes {
    pub active: Vec<Volume>,
    pub available: Vec<Volume>,
}

static VOLUMES: LazyLock<Volumes> = LazyLock::new(|| Volumes {
    active: vec![
        Volume {
            device_path: "/dev/sda1".to_owned(),
            mount_path: "/mnt/sda1".into(),
        },
        Volume {
            device_path: "/dev/sda2".to_owned(),
            mount_path: "/mnt/sda2".into(),
        },
    ],
    available: vec![
        Volume {
            device_path: "/dev/sda3".to_owned(),
            mount_path: "/mnt/sda3".into(),
        },
        Volume {
            device_path: "/dev/sda4".to_owned(),
            mount_path: "/mnt/sda4".into(),
        },
    ],
});

#[derive(Debug, Default)]
pub struct Example;

#[interface(
    name = "org.myservice.Example",
    proxy(
        default_path = "/org/myservice/Example",
        default_service = "org.myservice.Example",
        gen_blocking = false,
    )
)]
impl Example {
    #[zbus(property)]
    pub fn volumes(&self) -> Volumes {
        VOLUMES.clone()
    }

    pub fn parent(&self, path: &ZPath) -> ZPathBuf {
        path.parent().unwrap_or(path).into()
    }

    pub fn echo(&self, path: ZPathBuf) -> ZPathBuf {
        path
    }

    pub fn echo_borrowed(&self, path: &ZPath) -> ZPathBuf {
        path.to_owned()
    }
}
