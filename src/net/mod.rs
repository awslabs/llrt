mod socket;

use once_cell::sync::Lazy;
use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};
use rustls::{crypto::aws_lc_rs, ClientConfig, RootCertStore};
use webpki_roots::TLS_SERVER_ROOTS;

pub static TLS_CONFIG: Lazy<ClientConfig> = Lazy::new(|| {
    let mut root_certificates = RootCertStore::empty();

    for cert in TLS_SERVER_ROOTS.iter().cloned() {
        root_certificates.roots.push(cert)
    }

    ClientConfig::builder_with_provider(aws_lc_rs::default_provider().into())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_certificates)
        .with_no_client_auth()
});

pub struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        socket::declare(declare)?;
        declare.declare_static(cstr!("default"))?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        socket::init(ctx.clone(), exports)?;
        Ok(())
    }
}
