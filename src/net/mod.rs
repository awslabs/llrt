mod socket;

use once_cell::sync::Lazy;
use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};
use rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore};
use webpki::TrustAnchor;
use webpki_roots::TLS_SERVER_ROOTS;

pub static TLS_CONFIG: Lazy<ClientConfig> = Lazy::new(|| {
    let mut root_certificates = RootCertStore::empty();
    let create_owned_trust_anchor = |ta: &TrustAnchor| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    };
    root_certificates
        .add_server_trust_anchors(TLS_SERVER_ROOTS.0.iter().map(create_owned_trust_anchor));

    let tls: ClientConfig = ClientConfig::builder()
        .with_safe_defaults()
        //.with_native_roots()
        .with_root_certificates(root_certificates)
        .with_no_client_auth();

    tls
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
