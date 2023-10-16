use std::{
    borrow::Borrow,
    rc::Rc,
    cell::{
	RefCell,
	Ref,
	RefMut
    }
};

pub type Ptr<T> = Rc<RefCell<T>>;

pub trait Make {
    type T;
    fn make(x:Self::T)->Self;
    fn refer(&self)->Self;
    fn yank(&self)->Ref<Self::T>;
    fn yank_mut(&self)->RefMut<Self::T>;
}

impl<T> Make for Ptr<T> {
    type T = T;

    fn make(x:Self::T)->Self {
	Rc::new(RefCell::new(x))
    }

    fn refer(&self)->Self {
	Rc::clone(self)
    }

    fn yank(&self)->Ref<Self::T> {
	let w0 : &RefCell<Self::T> = self.borrow();
	let w1 : Ref<Self::T> = w0.borrow();
	w1
    }

    fn yank_mut(&self)->RefMut<Self::T> {
	let w0 : &RefCell<Self::T> = self.borrow();
	let w1 : RefMut<Self::T> = w0.borrow_mut();
	w1
    }
}

pub trait Gettable {
    type T;
    fn get(&self)->Self::T;
}

pub trait Settable {
    type T;
    fn set(&self,x:Self::T);
}

pub trait Updatable {
    type T;

    fn update<F:Fn(Self::T)->Self::T>(&self,f:F);
}

impl<T> Updatable for Ptr<T> where T:Copy {
    type T = T;

    fn update<F:Fn(Self::T)->Self::T>(&self,f:F) {
	let mut w = self.yank_mut();
	*w = f(*w);
    }
}

impl<T> Gettable for Ptr<T> where T:Copy {
    type T = T;

    fn get(&self)->Self::T {
	*self.yank()
    }
}

impl<T> Settable for Ptr<T> where T:Copy {
    type T = T;

    fn set(&self,x:T) {
	*self.yank_mut() = x;
    }
}
