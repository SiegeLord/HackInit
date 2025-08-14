// TODO: Clean up, this should just be a single function.
use nalgebra::Point3;

#[derive(Copy, Clone, Debug)]
pub struct NodeAndScore
{
	idx: i32,
	f_score: f32,
}

impl NodeAndScore
{
	pub fn new(idx: i32, f_score: f32) -> NodeAndScore
	{
		NodeAndScore {
			idx: idx,
			f_score: f_score,
		}
	}
}

impl Ord for NodeAndScore
{
	fn cmp(&self, other: &Self) -> std::cmp::Ordering
	{
		// Reverse to make the heap a minheap
		self.f_score.partial_cmp(&other.f_score).unwrap().reverse()
	}
}

impl PartialOrd for NodeAndScore
{
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
	{
		Some(self.cmp(other))
	}
}

impl PartialEq for NodeAndScore
{
	fn eq(&self, other: &Self) -> bool
	{
		self == other
	}
}

impl Eq for NodeAndScore {}

pub trait Node
{
	fn get_pos(&self) -> Point3<f32>;
	fn get_neighbours(&self) -> &[i32];
}

pub struct AStarContext<'n, NodeT>
{
	open_set: std::collections::BinaryHeap<NodeAndScore>,
	came_from: Vec<i32>,
	cost: Vec<f32>,
	nodes: &'n [NodeT],
}

impl<'n, NodeT: Node> AStarContext<'n, NodeT>
{
	pub fn new(nodes: &'n [NodeT]) -> Self
	{
		AStarContext {
			open_set: std::collections::BinaryHeap::new(),
			came_from: vec![-1; nodes.len()],
			cost: vec![0.; nodes.len()],
			nodes: nodes,
		}
	}

	fn heuristic(&self, from: i32, to: i32) -> f32
	{
		(self.nodes[from as usize].get_pos() - self.nodes[to as usize].get_pos()).norm()
	}

	fn pos_to_idx(&self, pos: Point3<f32>) -> Option<i32>
	{
		let mut best_idx = None;
		let mut best_distance = std::f32::INFINITY;
		for (i, node) in self.nodes.iter().enumerate()
		{
			let distance = (pos - node.get_pos()).norm();
			if distance < best_distance
			{
				best_idx = Some(i as i32);
				best_distance = distance;
			}
		}
		best_idx
	}

	fn idx_to_pos(&self, idx: i32) -> Point3<f32>
	{
		self.nodes[idx as usize].get_pos()
	}

	/// N.B. this returns the path in reverse order.
	pub fn solve<C: Fn(&NodeT, &NodeT) -> f32>(
		&mut self, from: Point3<f32>, to: Point3<f32>, cost_fn: C,
	) -> Vec<Point3<f32>>
	{
		let from_idx = self.pos_to_idx(from).unwrap();
		let to_idx = self.pos_to_idx(to).unwrap();

		self.open_set.clear();
		for i in 0..self.came_from.len()
		{
			self.came_from[i] = -1;
			self.cost[i] = 1e6;
		}

		self.cost[from_idx as usize] = self.heuristic(from_idx, from_idx);
		self.came_from[from_idx as usize] = from_idx as i32;
		self.open_set.push(NodeAndScore::new(
			from_idx,
			self.heuristic(from_idx, from_idx),
		));

		let mut best_score_so_far = self.heuristic(from_idx, to_idx);
		let mut best_idx_so_far = -1;

		let to_idx = self.pos_to_idx(to).unwrap();
		while !self.open_set.is_empty()
		{
			let cur = self.open_set.pop().unwrap();
			//~ println!("Trying {:?}", cur);
			if cur.idx == to_idx
			{
				let mut cur_idx = to_idx;
				let mut path = vec![to];
				//~ println!("Start {:?} {:?}", from, to);
				loop
				{
					//~ println!("Reconstructing: {}", cur_idx);
					cur_idx = self.came_from[cur_idx as usize];
					path.push(self.idx_to_pos(cur_idx));
					if cur_idx == from_idx
					{
						//~ path.reverse();
						//~ println!("Path len {}", path.len());
						//~ println!("Done");
						return path;
					}
				}
			}

			for &next_idx in self.nodes[cur.idx as usize].get_neighbours()
			{
				let new_cost = self.cost[cur.idx as usize]
					+ cost_fn(
						&self.nodes[cur.idx as usize],
						&self.nodes[next_idx as usize],
					);
				if new_cost < self.cost[next_idx as usize]
				{
					let new_heuristic = self.heuristic(next_idx, to_idx);
					if new_heuristic < best_score_so_far
					{
						best_score_so_far = new_heuristic;
						best_idx_so_far = next_idx;
					}

					self.came_from[next_idx as usize] = cur.idx;
					self.cost[next_idx as usize] = new_cost;
					self.open_set
						.push(NodeAndScore::new(next_idx, new_cost + new_heuristic));
				}
			}
		}
		if best_idx_so_far > -1
		{
			let mut cur_idx = best_idx_so_far;
			let mut path = vec![self.idx_to_pos(cur_idx)];
			//~ println!("Start {:?} {:?}", from, to);
			loop
			{
				//~ println!("Reconstructing: {}", cur_idx);
				cur_idx = self.came_from[cur_idx as usize];
				path.push(self.idx_to_pos(cur_idx));
				if cur_idx == from_idx
				{
					//~ path.reverse();
					//~ println!("Path len {}", path.len());
					//~ println!("Done");
					return path;
				}
			}
		}
		else
		{
			vec![]
		}
	}
}
