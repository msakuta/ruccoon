mod render;

use std::{cell::RefCell, cmp::Reverse, collections::BinaryHeap, error::Error, rc::Rc};

use eframe::epaint::{pos2, Color32, Pos2, Vec2};
use rand::{rngs::ThreadRng, Rng};
use ruscal::{
    ast::TypeDecl,
    bytecode::{ByteCode, NativeFn},
    compiler::Compiler,
    file_io::parse_program,
    type_checker::{type_check, TypeCheckContext},
    value::Value,
    vm::{Vm, YieldResult},
    Args,
};

use crate::app::{Hole, MapCell, BOARD_SIZE, BOARD_SIZE_I, CELL_SIZE_F};

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
    vm: Rc<RefCell<Vm>>,
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
        bytecode: &Rc<ByteCode>,
        debug_output: bool,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let gen_channel = |rng: &mut ThreadRng| rng.gen::<u8>() / 2 + 127;
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
        }));

        Self {
            id,
            state: state.clone(),
            vm: Rc::new(RefCell::new(Vm::new(
                bytecode.clone(),
                Box::new(VmUserData {
                    state,
                    map: map.clone(),
                    items: items.clone(),
                    holes: holes.clone(),
                }),
                debug_output,
            ))),
        }
    }

    pub(crate) fn animate(
        &self,
        others: &[Raccoon],
        map: &Rc<Vec<MapCell>>,
        items: &Rc<RefCell<Vec<Pos2>>>,
        holes: &Rc<Vec<Hole>>,
    ) {
        let mut vm = self.vm.borrow_mut();
        if vm.top().is_err() {
            if let Err(e) = vm.init_fn("main", &[]) {
                eprintln!("Error in raccoon {}: init_fn: {e}", self.id);
            }
        }

        let direction_code = match vm.interpret() {
            Ok(YieldResult::Finished(_)) => None,
            Ok(YieldResult::Suspend(res)) => res.coerce_i64().ok(),
            Err(e) => {
                eprintln!("Error in raccoon {}: {e}", self.id);
                None
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
        if let Some(direction) = direction_code.and_then(|code| DIRECTIONS.get(code as usize)) {
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

pub(crate) fn compile_program(args: &Args) -> Result<ByteCode, Box<dyn Error>> {
    let src = args.source.as_ref().expect("Source file exists");
    let source = std::fs::read_to_string(src).expect("Source file could be read");
    let ast = parse_program(src, &source).expect("Source parsed");

    let mut type_check_context = TypeCheckContext::new();
    extend_funcs(|name, func| type_check_context.add_fn(name, func));
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

    if args.disasm {
        compiler.disasm(&mut std::io::stdout())?;
    }

    let mut bytecode = compiler.into_bytecode();
    extend_funcs(|name, func| bytecode.add_fn(name, func));

    Ok(bytecode)
}

fn get_prop_fn(get: fn(&RaccoonState) -> i64) -> NativeFn<'static> {
    NativeFn::new(
        vec![],
        TypeDecl::I64,
        Box::new(move |state, _| {
            if let Some(data) = state.downcast_ref::<VmUserData>() {
                Value::I64(get(&data.state.borrow()))
            } else {
                Value::I64(0)
            }
        }),
    )
}

fn get_prop_fn_f(get: fn(&RaccoonState) -> f64) -> NativeFn<'static> {
    NativeFn::new(
        vec![],
        TypeDecl::F64,
        Box::new(move |data, _| {
            if let Some(data) = data.downcast_ref::<VmUserData>() {
                Value::F64(get(&data.state.borrow()))
            } else {
                Value::F64(0.)
            }
        }),
    )
}

fn extend_funcs(mut proc: impl FnMut(String, NativeFn<'static>)) {
    proc("get_x".to_string(), get_prop_fn(|state| state.pos.x as i64));
    proc("get_y".to_string(), get_prop_fn(|state| state.pos.y as i64));
    proc(
        "find_path_to_corn".to_string(),
        NativeFn::new(
            vec![],
            TypeDecl::I64,
            Box::new(move |state, _| {
                if let Some(data) = state.downcast_ref::<VmUserData>() {
                    let mut state = data.state.borrow_mut();
                    state.path = find_path(
                        [state.pos.x as i32, state.pos.y as i32],
                        &data.map,
                        &data.items.borrow(),
                    );
                    Value::I64(state.path.is_some() as i64)
                } else {
                    Value::I64(0)
                }
            }),
        ),
    );
    proc(
        "find_path_to_hole".to_string(),
        NativeFn::new(
            vec![],
            TypeDecl::I64,
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
                    state.path =
                        find_path([state.pos.x as i32, state.pos.y as i32], &data.map, &holes);
                    Value::I64(state.path.is_some() as i64)
                } else {
                    Value::I64(0)
                }
            }),
        ),
    );
    proc(
        "is_at_hole".to_string(),
        NativeFn::new(
            vec![],
            TypeDecl::I64,
            Box::new(move |state, _| {
                if let Some(data) = state.downcast_ref::<VmUserData>() {
                    let state = data.state.borrow();
                    Value::I64((data.holes.iter().any(|hole| state.pos == hole.pos)) as i64)
                } else {
                    Value::I64(0)
                }
            }),
        ),
    );
    proc(
        "get_next_move".to_string(),
        NativeFn::new(
            vec![],
            TypeDecl::I64,
            Box::new(move |state, _| {
                if let Some(data) = state.downcast_ref::<VmUserData>() {
                    let mut state = data.state.borrow_mut();
                    if let Some(node) = state.path.as_mut().and_then(|path| path.pop()) {
                        // println!("get_next_move returning {}", node.direction);
                        return Value::I64(node.direction as i64);
                    }
                }
                Value::I64(5)
            }),
        ),
    );
    proc(
        "get_satiety".to_string(),
        get_prop_fn_f(|state| state.satiety as f64),
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
