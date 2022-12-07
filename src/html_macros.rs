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
macro_rules! sql_struct {
	(
		$(#[$main_attr:meta] )*
		Table($table:literal)
		ID($id_name:literal)
		$visibility:vis struct $struct:ident <$sql:ty>
		{
			$id: ty,
			$(
				$(#[$field_attr:meta] )*
				$element_visibility:vis $element:ident: $type:ty
			),* $(,)?
		}
	)=>
	{

		$(#[$main_attr] )*
		#[derive(Debug)]
		#[derive(Serialize)]
		#[allow(dead_code)]
		#[derive(sqlx::FromRow)]
		$visibility struct $struct{
			#[sqlx(rename=$id_name)]
			pub id:$id,
			$(
				$(#[$field_attr] )*
				$element_visibility $element: $type,
			)*
		}


		pub enum Fields
		{
			id,
			$(
				$element
			),*
		}




		#[allow(dead_code)]
		impl $struct{
			pub fn new($($element: $type),*)->$struct{
				$struct{
					id: Default::default(),
					$(
						$element
					),*
				}
			}

			pub fn get_table(&self)->String{String::from($table)}
			//pub fn get_id_name(&self)->String{String::from($id_name)}
		}



		#[async_trait]
		impl Queryable<$struct, $id, $sql, Fields> for $struct{
			fn get_insert_string(&self)->Result<String, String>{
				let mut args = Vec::new();

				$(
					args.push(self.sql_types_string(Fields::$element));
				)*


				let sql_str = stringify!($sql);
				if sql_str.eq("Sqlite"){
					Ok(format!(r"INSERT INTO {} ({}) VALUES ({}) RETURNING *",$table,self.get_fields().connect(", "),
					args.connect(","))
				)
				}else if sql_str.eq("Mssql"){
					Ok(format!("INSERT INTO {} OUTPUT inserted.* VALUES ({})",$table, args.connect(",")))
				}else{
					panic!("{}",format!("Database pool '{}' is not yet implemented", sql_str));
					Err(format!("Database pool '{}' is not yet implemented", sql_str))
				}
			}

			fn get_update_string(&self)->Result<String, String>{
				let mut args = Vec::new();
				$(
					args.push(format!("{}={}",
					stringify!($element),&self.sql_types_string(Fields::$element)));
				)*

				let sql_str = stringify!($sql);
				if sql_str.eq("Sqlite"){
					Ok(format!("UPDATE {} SET {} WHERE {} = {} RETURNING *",$table,args.connect(", "), $id_name,
						&self.sql_types_string(Fields::id)))

				}else if sql_str.eq("Mssql"){
					Ok(format!("UPDATE {} SET {} OUTPUT inserted.* WHERE {}={}",$table,args.connect(", "), $id_name,
					&self.sql_types_string(Fields::id)))
				}else{
					panic!("{}",format!("Database pool '{}' is not yet implemented", sql_str));
					Err(format!("Database pool '{}' is not yet implemented", sql_str))
				}
			}


			fn get_fields(&self)->Vec<String>{ vec![$(stringify!($element).to_string()),*] }

			async fn get_one(db: &mut PoolConnection<$sql>,id: $id)->Result<$struct, sqlx::Error>{
				let q = format!("SELECT * FROM {} WHERE {} = {}",$table,$id_name,id);
                sqlx::query_as::<_, $struct>(q.as_str())
                	.fetch_one(&mut *db).await
			}

			async fn get_all(db: &mut PoolConnection<$sql>)->Result<Vec<$struct>,sqlx::Error>{
				let q = format!("SELECT * FROM {}",$table);
                sqlx::query_as::<_, $struct>(q.as_str())
                	.fetch_all(db).await
			}

			async fn insert(self,db: &mut PoolConnection<$sql>)->Result<$struct,sqlx::Error>{
					let q = self.get_insert_string().unwrap();
					sqlx::query_as::<_,$struct>(q.as_str()).fetch_one(db).await
			}

			async fn update(&self,db: &mut PoolConnection<$sql>)->Result<(), sqlx::Error>{
				let q = self.get_update_string().unwrap();
				match sqlx::query_as::<_,$struct>(q.as_str()).fetch_one(db).await{
					Ok(_)=>Ok(()),
					Err(err)=>Err(err),
				}
			}
		}
	}
}