use inkwell::context::Context;

use jet::{
    builder,
    builder::env::{Mode::Debug, Options},
    engine::Engine,
};

#[test]
fn invalid_opcode_returns_error() {
    let ctx = Context::create();
    let opts = Options::new(Debug, false, true);
    let mut engine = Engine::new(&ctx, opts).expect("engine initializes");

    let err = engine
        .build_contract("0x0000", &[0x0c])
        .expect_err("invalid opcode should fail");

    match err {
        jet::engine::Error::Build(builder::Error::InvalidOpcode(inner)) => {
            assert_eq!(inner.pc, 0);
            assert_eq!(inner.opcode, 0x0c);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
