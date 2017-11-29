#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(refcell_replace_swap)]

use std::collections::HashMap;
use std::cell::Cell;
use std::rc::Rc;
use std::any::Any;
use std::cell::RefCell;
use std::default::Default;
use std::borrow::BorrowMut;

trait Val {
    type Output;

    fn get(&self) -> Self::Output;
}

#[derive(Debug,Clone, Eq, Ord, PartialOrd, PartialEq, Default)]
struct NumVal {
    v: i64,
}

impl Val for NumVal {
    type Output = i64;

    fn get(&self) -> Self::Output {
        self.v
    }
}

impl std::ops::Add for NumVal {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            v: self.v + rhs.v
        }
    }
}

#[derive(Debug,Clone, Eq, Ord, PartialOrd, PartialEq, Default)]
struct BoolVal {
    v: bool,
}

impl Val for BoolVal {
    type Output = bool;

    fn get(&self) -> Self::Output {
        self.v
    }
}

trait Exp {
    type Output;

    fn stage(&self) -> Box<StagedExp<Output=Self::Output>>;

    fn interpret(&self) -> Self::Output;
}

trait StagedExp {
    type Output;

    fn run(&self) -> Self::Output;
}

struct ConstantExp<T: 'static+Clone> {
    const_val: T,
}

struct ConstantStagedExp<T: 'static+Clone> {
    const_val: T,
}

impl<T: 'static+Clone> Exp for ConstantExp<T>{
    type Output = T;

    fn stage(&self) -> Box<StagedExp<Output=Self::Output>> {
        box ConstantStagedExp {
            const_val: self.const_val.clone()
        }
    }
    fn interpret(&self) -> Self::Output {
        self.const_val.clone()
    }
}

impl<T: 'static+Clone> StagedExp for ConstantStagedExp<T>{
    type Output = T;

    fn run(&self) -> Self::Output {
        self.const_val.clone()
    }
}

static mut var_counter: i32 = 0;

#[derive(Debug,Clone)]
struct VariableExp<T: 'static+Clone> {
    id: i32,
    var_val: Rc<RefCell<T>>,
}

impl<T: 'static+Clone+Default> VariableExp<T> {
    fn fresh() -> VariableExp<T> {
        VariableExp {
            id: {
                unsafe{
                    var_counter += 1;
                    var_counter
                }
            },
            var_val: Rc::new(RefCell::new(T::default())),
        }
    }
}

impl<T: 'static+Clone> VariableExp<T> {
    fn fresh_with_val(v: T) -> VariableExp<T> {
        VariableExp {
            id: {
                unsafe{
                    var_counter += 1;
                    var_counter
                }
            },
            var_val: Rc::new(RefCell::new(v)),
        }
    }
}

impl<T: 'static+Clone> Exp for VariableExp<T>{
    type Output = T;

    fn stage(&self) -> Box<StagedExp<Output=Self::Output>> {
        box self.clone()
    }
    fn interpret(&self) -> Self::Output {
        self.var_val.borrow().clone()
    }
}

impl<T: 'static+Clone> StagedExp for VariableExp<T>{
    type Output = T;

    fn run(&self) -> Self::Output {
        self.var_val.borrow().clone()
    }
}

struct AddExp {
    exp1: Box<Exp<Output=NumVal>>,
    exp2: Box<Exp<Output=NumVal>>,
}

struct AddStagedExp {
    staged_exp1: Box<StagedExp<Output=NumVal>>,
    staged_exp2: Box<StagedExp<Output=NumVal>>,
}

impl Exp for AddExp{
    type Output = NumVal;

    fn stage(&self) -> Box<StagedExp<Output=Self::Output>> {
        box AddStagedExp {
            staged_exp1: self.exp1.stage(),
            staged_exp2: self.exp2.stage(),
        }
    }
    fn interpret(&self) -> Self::Output {
        self.exp1.interpret() + self.exp2.interpret()
    }
}

impl StagedExp for AddStagedExp{
    type Output = NumVal;

    fn run(&self) -> Self::Output {
        self.staged_exp1.run() + self.staged_exp2.run()
    }
}

struct LessThanExp {
    exp1: Box<Exp<Output=NumVal>>,
    exp2: Box<Exp<Output=NumVal>>,
}

struct LessThanStagedExp {
    staged_exp1: Box<StagedExp<Output=NumVal>>,
    staged_exp2: Box<StagedExp<Output=NumVal>>,
}

impl Exp for LessThanExp{
    type Output = BoolVal;

    fn stage(&self) -> Box<StagedExp<Output=Self::Output>> {
        box LessThanStagedExp {
            staged_exp1: self.exp1.stage(),
            staged_exp2: self.exp2.stage(),
        }
    }
    fn interpret(&self) -> Self::Output {
        Self::Output {
            v: self.exp1.interpret() < self.exp2.interpret()
        }
    }
}

impl StagedExp for LessThanStagedExp{
    type Output = BoolVal;

    fn run(&self) -> Self::Output {
        Self::Output {
            v: self.staged_exp1.run() < self.staged_exp2.run()
        }
    }
}

struct LetExp<T: 'static+Clone, U: 'static+Clone> {
    exp1: Box<Exp<Output=T>>,
    exp2: Box<Fn(VariableExp<T>) -> Box<Exp<Output=U>>>
}

struct LetStagedExp<T: 'static+Clone, U: 'static+Clone> {
    staged_exp1: Box<StagedExp<Output=T>>,
    staged_exp1_var: VariableExp<T>,
    staged_exp2: Box<StagedExp<Output=U>>,
}

impl<T: 'static+Clone+Default, U: 'static+Clone> Exp for LetExp<T,U>{
    type Output = U;

    fn stage(&self) -> Box<StagedExp<Output=Self::Output>> {
        let exp1_var = VariableExp::fresh();
        let staged_exp2 = (self.exp2)(exp1_var.clone()).stage();
        box LetStagedExp {
            staged_exp1: self.exp1.stage(),
            staged_exp1_var: exp1_var,
            staged_exp2,
        }
    }

    fn interpret(&self) -> Self::Output {
        let exp1_var = VariableExp::fresh_with_val(self.exp1.interpret());
        (self.exp2)(exp1_var).interpret()
    }
}

impl<T: 'static+Clone, U: 'static+Clone> StagedExp for LetStagedExp<T,U>{
    type Output = U;

    fn run(&self) -> Self::Output {
        self.staged_exp1_var.var_val.replace( self.staged_exp1.run() );
        self.staged_exp2.run()
    }
}

fn unit_exp<T: 'static+Clone>(const_val: T) -> ConstantExp<T> {
    ConstantExp {
        const_val
    }
}

fn add_exp(exp1: Box<Exp<Output=NumVal>>, exp2: Box<Exp<Output=NumVal>>) -> AddExp {
    AddExp {
        exp1,
        exp2
    }
}

fn let_exp<T: 'static+Clone+Default, U: 'static+Clone>(exp1: Box<Exp<Output=T>>,
                                                       exp2: Box<Fn(VariableExp<T>) -> Box<Exp<Output=U>>>) -> LetExp<T,U> {
    LetExp {
        exp1,
        exp2
    }
}

fn main() {
    // let i = 1 {
    //   while i < 1000 {
    //     i = i + 1
    //   }
    // }
//    let expr = Expr::Let(
//        "i",
//        Type::Number,
//        box Expr::Constant(Value::Number(1)),
//        box Expr::While(
//            box Expr::LessThan(box Expr::Get("i"), box Expr::Constant(Value::Number(1000))),
//            box Expr::Set(
//                "i",
//                box Expr::Add(box Expr::Get("i"), box Expr::Constant(Value::Number(1))),
//            ),
//        ),
//    );
//
//    println!("{:?}", interpret(&HashMap::new(), &expr));
//    if let Staged::Bool(bool) = stage(&HashMap::new(), &expr) {
//        println!("{:?}", bool());
//    }

    let num1 = unit_exp(NumVal{ v: 1 });
    let num2 = unit_exp(NumVal{ v: 2 });
    let add_nums = add_exp(box num1, box num2);
    let let_nums = let_exp(box add_nums, box |v| {
        let num3 = unit_exp(NumVal{ v: 5 });
        let add_nums2 = add_exp(box v, box num3);
        box add_nums2
    });

    println!("{:?}", let_nums.interpret());

    let staged_expr = let_nums.stage();
    println!("{:?}", staged_expr.run());
}
