#[macro_export]
macro_rules! html {
    ($($x:expr),*) => {
		{
			let mut s = String::new();
			$(
					s+=&format!("{}",&$x);
				)*
			RawHtml(format!("<!DOCTYPE html>{}",tag!(html, s)))
		}
	};
}

#[macro_export]
macro_rules! tag_str {
    ($h:expr $(, $x:expr)*) => {
		{
			let mut all = String::new();
			$(
				all+=&format!("{}",&(&$x));
			)*

			let s = match ($h.chars().position(|c| c==' ')){
				Some(index)=>{
					let big_expression = &$h[..index];
					format!("<{}>{}</{}>",$h,all,big_expression)
				},
				None=>{
					format!("<{}>{}</{}>",$h,all,$h)
				}
			};
			s
		}
	};
}

#[macro_export]
macro_rules! tag {
	($($h :expr)+ $(, $x:expr)*) => {
		{
			let mut exp = String::new();
			$(
				exp+=&format!("{} ",stringify!($h));
			)+


			tag_str!(exp $(, $x)*)
		}
	};
}

#[macro_export]
macro_rules! create {
	(
		#[Table($table:literal)]
		$visibility:vis struct $struct:ident <$sql:ty>
		{
			$id: ty,
			$($element_visibility:vis $element:ident: $type:ty),*
		}
	)=>
	{



		#[derive(Debug)]
		#[allow(dead_code)]
		#[derive(sqlx::FromRow)]
		$visibility struct $struct{
			pub ID:$id,
			$(
				$element_visibility $element: $type,
			)*
		}

		#[allow(dead_code)]
		impl $struct{
/*			pub fn new($($element: $type),*)->$struct{
				$struct{
					ID: Default::default(),
					$(
						$element
					),*
				}
			}*/

			pub fn get_table(&self)->String{String::from($table)}
		}


		#[async_trait]
		impl Queryable<$struct, $id, $sql> for $struct{
			async fn get_one(db: &mut PoolConnection<$sql>,id: $id)->Option<$struct>{
				let q = format!("SELECT * FROM {} WHERE ID = {}",$table,id);
                sqlx::query_as::<_, $struct>(q.as_str())
                	.fetch_one(&mut *db).await.ok()
			}

			async fn get_all(db: &mut PoolConnection<$sql>)->Vec<$struct>{
				let q = format!("SELECT * FROM {}",$table);
                sqlx::query_as::<_, $struct>(q.as_str())
                	.fetch_all(db).await.ok().unwrap()
			}

			async fn insert(self,db: &mut PoolConnection<$sql>)->$struct{
					let q = self.get_insert_string();
					sqlx::query_as::<_,$struct>(q.as_str()).fetch_one(&mut *db).await.ok().unwrap()
			}

			async fn update(&self,db: &mut PoolConnection<$sql>){
				let q = self.get_update_string();
				sqlx::query_as::<_,$struct>(q.as_str()).fetch_one(&mut *db).await;
			}
		}
	}
}
