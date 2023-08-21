use super::util;
use crate::{
    ijson, CountQueryExt, Database, Datastore, Edge, Error, Identifier, NamedProperty, QueryExt, QueryOutputValue,
    SpecificVertexQuery, Vertex, VertexProperties,
};

pub fn should_get_nested_include_query<D: Datastore>(db: &Database<D>) -> Result<(), Error> {
    let (outbound_id, inbound_ids) = util::create_edges(db)?;
    let q = SpecificVertexQuery::single(outbound_id)
        .include()
        .outbound()?
        .include()
        .count()?;
    let output = db.get(q)?;
    assert_eq!(
        output,
        vec![
            QueryOutputValue::Vertices(vec![Vertex::with_id(
                outbound_id,
                Identifier::new("https://example.org/test_outbound_vertex_type")?
            )]),
            QueryOutputValue::Edges(
                inbound_ids
                    .into_iter()
                    .map(|id| Edge::new(outbound_id, Identifier::new("https://example.org/test_edge_type").unwrap(), id))
                    .collect()
            ),
            QueryOutputValue::Count(5),
        ]
    );
    Ok(())
}

pub fn should_get_unnested_include_query<D: Datastore>(db: &Database<D>) -> Result<(), Error> {
    let id = db.create_vertex_from_type(Identifier::new("https://example.org/foo")?)?;
    let q = SpecificVertexQuery::single(id);
    db.set_properties(q.clone(), &Identifier::new("https://example.org/bar")?, &ijson!(true))?;
    let output = db.get(q.include().properties()?)?;
    assert_eq!(
        output,
        vec![
            QueryOutputValue::Vertices(vec![Vertex::with_id(id, Identifier::new("https://example.org/foo")?)]),
            QueryOutputValue::VertexProperties(vec![VertexProperties::new(
                Vertex::with_id(id, Identifier::new("https://example.org/foo")?),
                vec![NamedProperty::new(Identifier::new("https://example.org/bar")?, ijson!(true)),],
            )])
        ]
    );
    Ok(())
}

pub fn should_include_with_property_presence<D: Datastore>(db: &Database<D>) -> Result<(), Error> {
    let id = db.create_vertex_from_type(Identifier::new("https://example.org/foo")?)?;
    let q = SpecificVertexQuery::single(id);
    db.index_property(&Identifier::new("https://example.org/bar")?)?;
    db.set_properties(q.clone(), &Identifier::new("https://example.org/bar")?, &ijson!(true))?;
    let output = db.get(q.clone().include().with_property(Identifier::new("https://example.org/bar")?)?)?;
    assert_eq!(
        output,
        vec![
            QueryOutputValue::Vertices(vec![Vertex::with_id(id, Identifier::new("https://example.org/foo")?)]),
            QueryOutputValue::Vertices(vec![Vertex::with_id(id, Identifier::new("https://example.org/foo")?)]),
        ]
    );
    let output = db.get(
        q.include()
            .with_property_equal_to(Identifier::new("https://example.org/bar")?, ijson!(true))?,
    )?;
    assert_eq!(
        output,
        vec![
            QueryOutputValue::Vertices(vec![Vertex::with_id(id, Identifier::new("https://example.org/foo")?)]),
            QueryOutputValue::Vertices(vec![Vertex::with_id(id, Identifier::new("https://example.org/foo")?)]),
        ]
    );
    Ok(())
}
