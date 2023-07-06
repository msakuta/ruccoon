use std::{cell::RefCell, error::Error, rc::Rc};

use eframe::epaint::{pos2, Color32, Pos2, Vec2};
use rand::{seq::SliceRandom, Rng};
use ruscal::{
    ast::TypeDecl,
    bytecode::{ByteCode, NativeFn},
    compiler::Compiler,
    file_io::parse_program,
    type_checker::{type_check, TypeCheckContext},
    value::Value,
    vm::Vm,
    Args,
};

use crate::app::BOARD_SIZE;

pub(crate) struct Rascal {
    id: usize,
    pub(crate) state: Rc<RefCell<RascalState>>,
    vm: Vm,
}

pub(crate) struct RascalState {
    pub(crate) pos: Pos2,
    pub(crate) tint: Color32,
}

impl Rascal {
    pub(crate) fn new(id: usize, bytecode: &Rc<ByteCode>) -> Self {
        let mut rng = rand::thread_rng();
        let state = Rc::new(RefCell::new(RascalState {
            pos: pos2(
                rng.gen_range(0..BOARD_SIZE) as f32,
                rng.gen_range(0..BOARD_SIZE) as f32,
            ),
            tint: Color32::from_rgb(rng.gen(), rng.gen(), rng.gen()),
        }));

        Self {
            id,
            state: state.clone(),
            vm: Vm::new(bytecode.clone(), Box::new(state)),
        }
    }

    pub(crate) fn animate(&mut self) {
        const DIRECTIONS: [Vec2; 4] = [
            Vec2::new(-1., 0.),
            Vec2::new(0., -1.),
            Vec2::new(1., 0.),
            Vec2::new(0., 1.),
        ];
        if let Some(direction) = DIRECTIONS.choose(&mut rand::thread_rng()) {
            let mut state = self.state.borrow_mut();
            state.pos += *direction;
            if state.pos.x < 0. {
                state.pos.x = 0.;
            } else if BOARD_SIZE as f32 <= state.pos.x {
                state.pos.x = (BOARD_SIZE - 1) as f32;
            }
            if state.pos.y < 0. {
                state.pos.y = 0.;
            } else if BOARD_SIZE as f32 <= state.pos.y {
                state.pos.y = (BOARD_SIZE - 1) as f32;
            }
        }

        if self.vm.top().is_err() {
            if let Err(e) = self.vm.init_fn("main", &[]) {
                eprintln!("Error in rascal {}: init_fn: {e}", self.id);
            }
        }

        if let Err(e) = self.vm.interpret() {
            eprintln!("Error in rascal {}: {e}", self.id);
        }
    }
}

pub(crate) fn compile_program(args: &Args) -> Result<ByteCode, Box<dyn Error>> {
    let src = args.source.as_ref().expect("Source file exists");
    let source = std::fs::read_to_string(src).expect("Source file could be read");
    let ast = parse_program(src, &source).expect("Source parsed");

    fn get_prop_fn(get: fn(&RascalState) -> i64) -> NativeFn<'static> {
        NativeFn::new(
            vec![],
            TypeDecl::I64,
            Box::new(move |state, _| {
                if let Some(state) = state.downcast_ref::<Rc<RefCell<RascalState>>>() {
                    Value::I64(get(&state.borrow()))
                } else {
                    Value::I64(0)
                }
            }),
        )
    }

    let mut type_check_context = TypeCheckContext::new();
    type_check_context.add_fn("get_x".to_string(), get_prop_fn(|state| state.pos.x as i64));
    type_check_context.add_fn("get_y".to_string(), get_prop_fn(|state| state.pos.y as i64));
    match type_check(&ast, &mut type_check_context) {
        Ok(_) => println!("Typecheck Ok"),
        Err(e) => {
            return Err(format!(
                "{}:{}:{}: {}",
                src,
                e.span.location_line(),
                e.span.get_utf8_column(),
                e
            )
            .into())
        }
    }

    let mut compiler = Compiler::new();
    compiler.compile(&ast)?;

    let mut bytecode = compiler.into_bytecode();
    bytecode.add_fn("get_x".to_string(), get_prop_fn(|state| state.pos.x as i64));
    bytecode.add_fn("get_y".to_string(), get_prop_fn(|state| state.pos.y as i64));

    Ok(bytecode)
}
