mod render;

use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    path::Path,
    rc::Rc,
};

use anyhow::Context;
use eframe::epaint::{Color32, Pos2, Vec2, pos2};
use mascal::{
    Bytecode, CompilerBuilder, FuncDef, NativeCode, NativeFn, TypeCheckContext, TypeCheckError,
    TypeDecl, Value, Vm, type_check,
};
use rand::{Rng, rngs::ThreadRng};

use crate::app::{BOARD_SIZE, BOARD_SIZE_I, CELL_SIZE_F, Hole, MapCell};

const DIRECTIONS: [Vec2; 4] = [
    Vec2::new(-1., 0.),
    Vec2::new(0., -1.),
    Vec2::new(1., 0.),
    Vec2::new(0., 1.),
];

const CORN_ENERGY: f32 = 0.2;
const HUNGER_RATE: f32 = 0.005;

pub(crate) struct Raccoon {
    id: usize,
    pub(crate) state: Rc<RefCell<RaccoonState>>,
    vm: Rc<RefCell<Vm<'static>>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PathNode {
    direction: u8,
    pos: [i32; 2],
}

impl From<&PathNode> for Pos2 {
    fn from(value: &PathNode) -> Self {
        pos2(
            (value.pos[0] as f32 + 0.5) * CELL_SIZE_F,
            (value.pos[1] as f32 + 0.5) * CELL_SIZE_F,
        )
    }
}

pub(crate) struct RaccoonState {
    pub(crate) pos: Pos2,
    pub(crate) tint: Color32,
    pub(crate) path: Option<Vec<PathNode>>,
    pub(crate) ate: usize,
    pub(crate) satiety: f32,
    yielded: Option<i32>,
}

struct VmUserData {
    state: Rc<RefCell<RaccoonState>>,
    map: Rc<Vec<MapCell>>,
    items: Rc<RefCell<Vec<Pos2>>>,
    holes: Rc<Vec<Hole>>,
}

impl Raccoon {
    pub(crate) fn new(
        id: usize,
        map: &Rc<Vec<MapCell>>,
        items: &Rc<RefCell<Vec<Pos2>>>,
        holes: &Rc<Vec<Hole>>,
        make_bytecode: impl FnOnce() -> Bytecode,
    ) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();
        let gen_channel = |rng: &mut ThreadRng| rng.r#gen::<u8>() / 2 + 127;
        let state = Rc::new(RefCell::new(RaccoonState {
            pos: pos2(
                rng.gen_range(0..BOARD_SIZE) as f32,
                rng.gen_range(0..BOARD_SIZE) as f32,
            ),
            tint: Color32::from_rgb(
                gen_channel(&mut rng),
                gen_channel(&mut rng),
                gen_channel(&mut rng),
            ),
            path: None,
            ate: 0,
            satiety: 0.5,
            yielded: None,
        }));

        // Since both Vm and Bytecode would be a part of a Raccoon, they are technically
        // self-referencing, so we need to leak memory to allow static lifetime.
        let bytecode = Box::leak(Box::new(make_bytecode()));

        Ok(Self {
            id,
            state: state.clone(),
            vm: Rc::new(RefCell::new(
                Vm::start_main(
                    bytecode,
                    Rc::new(VmUserData {
                        state,
                        map: map.clone(),
                        items: items.clone(),
                        holes: holes.clone(),
                    }),
                    // debug_output,
                )
                .context("Creating Vm")?,
            )),
        })
    }

    pub(crate) fn animate(
        &self,
        others: &[Raccoon],
        map: &Rc<Vec<MapCell>>,
        items: &Rc<RefCell<Vec<Pos2>>>,
        holes: &Rc<Vec<Hole>>,
    ) {
        let mut vm = self.vm.borrow_mut();

        let direction_code = loop {
            let res = match vm.next_inst() {
                Ok(res) => res,
                Err(e) => {
                    println!("next_inst Error: {e}");
                    return;
                }
            };
            print!(".");
            if let Some(res) = res {
                print!("{res:?}");
            }
            let mut state = self.state.borrow_mut();
            if let Some(yielded) = state.yielded {
                println!("yielded!");
                state.yielded = None;
                break yielded;
            }
        };

        let is_blocked = |pos: Pos2| {
            if !matches!(
                map[pos.x as usize + pos.y as usize * BOARD_SIZE],
                MapCell::Empty(_)
            ) {
                return true;
            }
            if others.iter().any(|other| {
                let Ok(other_state) = other.state.try_borrow() else {
                    return false;
                };
                other_state.pos == pos
            }) {
                return true;
            }
            false
        };

        let prev_pos = self.state.borrow().pos;
        if let Some(direction) = DIRECTIONS.get(direction_code as usize) {
            let mut state = self.state.borrow_mut();
            let mut pos = state.pos + *direction;

            if pos.x < 0. {
                pos.x = 0.;
            } else if BOARD_SIZE as f32 <= pos.x {
                pos.x = (BOARD_SIZE - 1) as f32;
            }
            if pos.y < 0. {
                pos.y = 0.;
            } else if BOARD_SIZE as f32 <= pos.y {
                pos.y = (BOARD_SIZE - 1) as f32;
            }

            if !is_blocked(pos) {
                state.pos = pos;
            }
        }

        let mut state = self.state.borrow_mut();
        let mut items = items.borrow_mut();
        if let Some((i, _)) = items
            .iter()
            .enumerate()
            .find(|(_, item)| **item == state.pos)
        {
            items.remove(i);
            state.ate += 1;
            state.satiety += CORN_ENERGY;
            println!(
                "Raccoon {} ate {} corns and satiety became {}",
                self.id, state.ate, state.satiety
            );
        }

        // Getting hungry over time
        state.satiety = (state.satiety - HUNGER_RATE).max(0.).min(1.);

        if prev_pos != state.pos {
            if let Some(hole) = holes.iter().find(|hole| prev_pos == hole.pos) {
                hole.occupied.set(false);
            }
        }

        if let Some(hole) = holes.iter().find(|hole| state.pos == hole.pos) {
            hole.occupied.set(true);
        }
    }
}

#[derive(Debug)]
pub(crate) enum CompileError {
    TypeCheck(String),
    IO(std::io::Error),
    Compile(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TypeCheck(e) => write!(f, "Type check error: {e}"),
            Self::IO(e) => e.fmt(f),
            Self::Compile(e) => write!(f, "Compile error: {e}"),
        }
    }
}

impl std::error::Error for CompileError {}

impl<'a> From<TypeCheckError<'a>> for CompileError {
    fn from(value: TypeCheckError<'a>) -> Self {
        Self::TypeCheck(value.to_string())
    }
}

impl From<std::io::Error> for CompileError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl<'a> From<mascal::CompileError<'a>> for CompileError {
    fn from(value: mascal::CompileError<'a>) -> Self {
        Self::Compile(value.to_string())
    }
}

pub(crate) fn compile_program(src_file: &Path) -> Result<Bytecode, CompileError> {
    let src = std::fs::read_to_string(src_file).expect("Source file could be read");
    let (_, mut ast) = mascal::source(&src).expect("Source parsed");

    let mut type_check_context = TypeCheckContext::new(src_file.to_str());
    extend_funcs(|name, func, ret_ty| {
        type_check_context.set_fn(
            &name,
            FuncDef::Native(NativeCode::new(Box::leak(func), vec![], Some(ret_ty))),
        );
    });
    type_check_context.set_fn(
        "yield_",
        FuncDef::Native(NativeCode::new(&|_, _| Ok(Value::I64(0)), vec![], None)),
    );
    type_check(&mut ast, &mut type_check_context).map_err(CompileError::from)?;
    println!("Typecheck Ok");

    let mut functions = HashMap::new();
    extend_funcs(|name, func, _| {
        functions.insert(name, func);
    });
    functions.insert(
        "yield_".to_string(),
        Box::new(|ctx, args| {
            if let Some(user_data) = ctx.downcast_ref::<VmUserData>()
                && let Some(arg) = args.first()
                && let Ok(arg) = mascal::coercion::coerce_i32(arg)
            {
                println!("Yield flag set");
                user_data.state.borrow_mut().yielded = Some(arg);
            }
            Ok(Value::I32(0))
        }),
    );

    let compiler = CompilerBuilder::new(&ast).functions(functions);
    let bytecode = compiler.compile(&mut std::io::sink())?;

    // if args.disasm {
    //     compiler.disasm(&mut std::io::stdout())?;
    // }

    Ok(bytecode)
}

fn get_prop_fn(get: fn(&RaccoonState) -> i32) -> NativeFn {
    Box::new(move |state, _| {
        if let Some(data) = state.downcast_ref::<VmUserData>() {
            Ok(Value::I32(get(&data.state.borrow())))
        } else {
            Ok(Value::I32(0))
        }
    })
}

fn get_prop_fn_f(get: fn(&RaccoonState) -> f64) -> NativeFn {
    Box::new(move |data, _| {
        if let Some(data) = data.downcast_ref::<VmUserData>() {
            Ok(Value::F64(get(&data.state.borrow())))
        } else {
            Ok(Value::F64(0.))
        }
    })
}

fn extend_funcs(mut proc: impl FnMut(String, NativeFn, TypeDecl)) {
    proc(
        "get_x".to_string(),
        get_prop_fn(|state| state.pos.x as i32),
        TypeDecl::I32,
    );
    proc(
        "get_y".to_string(),
        get_prop_fn(|state| state.pos.y as i32),
        TypeDecl::I32,
    );
    proc(
        "find_path_to_corn".to_string(),
        Box::new(move |state, _| {
            if let Some(data) = state.downcast_ref::<VmUserData>() {
                let mut state = data.state.borrow_mut();
                state.path = find_path(
                    [state.pos.x as i32, state.pos.y as i32],
                    &data.map,
                    &data.items.borrow(),
                );
                Ok(Value::I32(state.path.is_some() as i32))
            } else {
                Ok(Value::I32(0))
            }
        }),
        TypeDecl::I32,
    );
    proc(
        "find_path_to_hole".to_string(),
        Box::new(move |state, _| {
            if let Some(data) = state.downcast_ref::<VmUserData>() {
                let mut state = data.state.borrow_mut();
                let holes: Vec<_> = data
                    .holes
                    .iter()
                    .filter_map(|hole| {
                        if hole.occupied.get() {
                            None
                        } else {
                            Some(hole.pos)
                        }
                    })
                    .collect();
                state.path = find_path([state.pos.x as i32, state.pos.y as i32], &data.map, &holes);
                Ok(Value::I32(state.path.is_some() as i32))
            } else {
                Ok(Value::I32(0))
            }
        }),
        TypeDecl::I32,
    );
    proc(
        "is_at_hole".to_string(),
        Box::new(move |state, _| {
            if let Some(data) = state.downcast_ref::<VmUserData>() {
                let state = data.state.borrow();
                Ok(Value::I32(
                    (data.holes.iter().any(|hole| state.pos == hole.pos)) as i32,
                ))
            } else {
                Ok(Value::I32(0))
            }
        }),
        TypeDecl::I32,
    );
    proc(
        "get_next_move".to_string(),
        Box::new(move |state, _| {
            if let Some(data) = state.downcast_ref::<VmUserData>() {
                let mut state = data.state.borrow_mut();
                if let Some(node) = state.path.as_mut().and_then(|path| path.pop()) {
                    println!("get_next_move returning {}", node.direction);
                    return Ok(Value::I64(node.direction as i64));
                }
            }
            Ok(Value::I64(5))
        }),
        TypeDecl::I64,
    );
    proc(
        "get_satiety".to_string(),
        get_prop_fn_f(|state| state.satiety as f64),
        TypeDecl::F64,
    );
}

fn find_path(start: [i32; 2], map: &[MapCell], items: &[Pos2]) -> Option<Vec<PathNode>> {
    // println!("finding path for {items:?}");
    let mut cost_map = [i32::MAX; BOARD_SIZE * BOARD_SIZE];
    let mut came_from: [Option<u8>; BOARD_SIZE * BOARD_SIZE] = [None; BOARD_SIZE * BOARD_SIZE];

    #[derive(Eq, Ord)]
    struct MinCost {
        pos: [i32; 2],
        cost: i32,
    }

    impl PartialEq for MinCost {
        fn eq(&self, other: &Self) -> bool {
            self.cost.eq(&other.cost)
        }
    }

    impl PartialOrd for MinCost {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Reverse(self.cost).partial_cmp(&Reverse(other.cost))
        }
    }

    let mut open_set = BinaryHeap::new();
    open_set.push(MinCost {
        pos: start,
        cost: 0,
    });
    cost_map[(start[0] + start[1] * BOARD_SIZE_I) as usize] = 0;
    while let Some(state) = open_set.pop() {
        if let Some(goal) = items
            .iter()
            .find(|item| [item.x as i32, item.y as i32] == state.pos)
        {
            let mut path = vec![PathNode {
                direction: 5,
                pos: [goal.x as i32, goal.y as i32],
            }];
            let mut cur = [goal.x as i32, goal.y as i32];
            while let Some(direction_idx) = came_from[(cur[0] + cur[1] * BOARD_SIZE_I) as usize] {
                let direction = DIRECTIONS[direction_idx as usize];
                let x = cur[0] + direction.x as i32;
                let y = cur[1] + direction.y as i32;
                path.push(PathNode {
                    direction: (direction_idx + 2) % 4,
                    pos: [x as i32, y as i32],
                });
                cur = [x as i32, y as i32];
            }
            // println!("find_path returning {path:?}");
            return Some(path);
        }
        let prev_cost = state.cost;
        for (direction, next) in DIRECTIONS.iter().enumerate() {
            let next = [state.pos[0] + next.x as i32, state.pos[1] + next.y as i32];
            if next[0] < 0 || BOARD_SIZE_I <= next[0] || next[1] < 0 || BOARD_SIZE_I <= next[1] {
                continue;
            }
            let idx = (next[0] + next[1] * BOARD_SIZE_I) as usize;
            if !matches!(map[idx], MapCell::Empty(_)) {
                continue;
            }
            let cost_cell = &mut cost_map[idx];
            if prev_cost + 1 < *cost_cell {
                open_set.push(MinCost {
                    pos: next,
                    cost: prev_cost + 1,
                });
                *cost_cell = prev_cost + 1;
                came_from[idx] = Some(((direction + 2) % 4) as u8);
            }
        }
    }
    println!("find_path returning None");
    None
}
