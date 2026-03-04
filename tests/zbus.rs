mod common;

use zpath::{ZPath, ZPathBuf};

use crate::common::TestClient;

macro_rules! zbus_test {
    (async fn $name:ident($client:ident) $body:block) => {
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn $name() {
            let $client = TestClient::new().await;
            $body
        }
    };
}

// helpers

async fn assert_parent(client: &TestClient<'_>, paths: &[(&[u8], &[u8])]) {
    for &(bytes, expected) in paths {
        let parent = client.parent(ZPath::from_bytes(bytes)).await.unwrap();
        assert_eq!(&parent, ZPath::from_bytes(expected));
    }
}

async fn assert_echo_roundtrip(client: &TestClient<'_>, paths: &[&[u8]]) {
    for &bytes in paths {
        let echo = client
            .echo(ZPathBuf::from_vec(bytes.to_vec()))
            .await
            .unwrap();
        assert_eq!(&echo, ZPath::from_bytes(bytes));
    }
}

async fn assert_echo_borrowed_roundtrip(client: &TestClient<'_>, paths: &[&[u8]]) {
    for &bytes in paths {
        let echo = client
            .echo_borrowed(ZPath::from_bytes(bytes))
            .await
            .unwrap();
        assert_eq!(&echo, ZPath::from_bytes(bytes));
    }
}

// property

zbus_test! {
    async fn property_volumes(client) {
        let vols = client.volumes().await.unwrap();

        assert_eq!(vols.active.len(), 2);
        assert_eq!(vols.available.len(), 2);
        assert_eq!(vols.active[0].mount_path, ZPathBuf::from("/mnt/sda1"));
        assert_eq!(vols.active[1].mount_path, ZPathBuf::from("/mnt/sda2"));
        assert_eq!(vols.available[0].mount_path, ZPathBuf::from("/mnt/sda3"));
        assert_eq!(vols.available[1].mount_path, ZPathBuf::from("/mnt/sda4"));
    }
}

// parent(&ZPath) -> ZPathBuf

zbus_test! {
    async fn method_zpath_valid_utf8(client) {
        assert_parent(&client, &[(b"/a/b/c/d", b"/a/b/c/")]).await;
    }
}

zbus_test! {
    async fn method_zpath_invalid_utf8(client) {
        assert_parent(
            &client,
            &[
                (b"/a/b/c/\xff\xfe", b"/a/b/c/"),
                (b"/a/\xff\xfe/c/d", b"/a/\xff\xfe/c/"),
            ],
        )
        .await;
    }
}

zbus_test! {
    async fn method_zpath_invalid_utf8_with_nul(client) {
        assert_parent(
            &client,
            &[
                (b"/a/b/c/\xff\xfe/e\x00e", b"/a/b/c/\xff\xfe"),
                (b"/a\x00/\xff\xfe/c/d", b"/a\x00/\xff\xfe/c/"),
                (b"/a\x00/\xff\xfe/c/d/\x00", b"/a\x00/\xff\xfe/c/d"),
            ],
        )
        .await;
    }
}

// echo(ZPathBuf) -> ZPathBuf

zbus_test! {
    async fn method_zpathbuf_valid_utf8(client) {
        assert_echo_roundtrip(&client, &[b"/a/b/c/d"]).await;
    }
}

zbus_test! {
    async fn method_zpathbuf_invalid_utf8(client) {
        assert_echo_roundtrip(&client, &[b"/a/b/c/\xff\xfe", b"/a/\xff\xfe/c/d"]).await;
    }
}

zbus_test! {
    async fn method_zpathbuf_invalid_utf8_with_nul(client) {
        assert_echo_roundtrip(
            &client,
            &[
                b"/a/b/c/\xff\xfe/e\x00e",
                b"/a\x00/\xff\xfe/c/d",
                b"/a\x00/\xff\xfe/c/d/\x00",
            ],
        )
        .await;
    }
}

// echo_borrowed(&ZPath) -> ZPathBuf

zbus_test! {
    async fn method_echo_borrowed_valid_utf8(client) {
        assert_echo_borrowed_roundtrip(&client, &[b"/a/b/c/d"]).await;
    }
}

zbus_test! {
    async fn method_echo_borrowed_invalid_utf8(client) {
        assert_echo_borrowed_roundtrip(&client, &[b"/a/b/c/\xff\xfe", b"/a/\xff\xfe/c/d"]).await;
    }
}

zbus_test! {
    async fn method_echo_borrowed_invalid_utf8_with_nul(client) {
        assert_echo_borrowed_roundtrip(
            &client,
            &[
                b"/a/b/c/\xff\xfe/e\x00e",
                b"/a\x00/\xff\xfe/c/d",
                b"/a\x00/\xff\xfe/c/d/\x00",
            ],
        )
        .await;
    }
}
