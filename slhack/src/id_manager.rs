pub struct IdManager<IdT>
{
	next_id: IdT,
	free: Vec<IdT>,
}

impl<IdT: Id> IdManager<IdT>
{
	pub fn new() -> Self
	{
		Self {
			next_id: Default::default(),
			free: vec![],
		}
	}

	pub fn get(&mut self) -> IdT
	{
		if let Some(id) = self.free.pop()
		{
			id
		}
		else
		{
			let mut id = self.next_id.next();
			std::mem::swap(&mut self.next_id, &mut id);
			id
		}
	}

	pub fn put(&mut self, id: IdT)
	{
		self.free.push(id);
	}
}

pub trait Id: Default + PartialEq
{
	fn next(&self) -> Self;
}

impl Id for u32
{
	fn next(&self) -> u32
	{
		*self + 1
	}
}

impl Id for usize
{
	fn next(&self) -> usize
	{
		*self + 1
	}
}
