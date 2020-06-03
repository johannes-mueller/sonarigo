

use lv2::prelude::*;

#[uri("http://lv2plug.in/ns/ext/patch#Set")]
pub struct PatchSet;

#[uri("http://lv2plug.in/ns/ext/patch#Get")]
pub struct PatchGet;

#[uri("http://lv2plug.in/ns/ext/patch#Put")]
pub struct PatchPut;

#[uri("http://lv2plug.in/ns/ext/patch#body")]
pub struct PatchBody;

#[uri("http://lv2plug.in/ns/ext/patch#property")]
pub struct PatchProperty;

#[uri("http://lv2plug.in/ns/ext/patch#value")]
pub struct PatchValue;

#[derive(URIDCollection)]
pub struct PatchURIDCollection {
    pub set: URID<PatchSet>,
    pub get: URID<PatchGet>,
    pub put: URID<PatchPut>,
    pub body: URID<PatchBody>,
    pub property: URID<PatchProperty>,
    pub value: URID<PatchValue>
}


/*
impl<'a, 'b> Atom<'a, 'b> for PatchValue
where
    'a: 'b,
{
    type ReadParameter = ();
    type ReadHandle = PatchSetReader<'a>;

    type WriteParameter = ();
    type WriteHandle = PatchSetWriter<'a, 'b>;

    fn read(body: Space<'a>, _: ()) -> Option<PatchSetReader> {
        println!("Value read");
        Some(PatchSetReader { space: body })
    }

    fn init(frame: FramedMutSpace<'a, 'b>, _: ()) -> Option<PatchSetWriter<'a, 'b>> {
        Some(PatchSetWriter { frame })
    }
}

impl<'a, 'b> Atom<'a, 'b> for PatchProperty
where
    'a: 'b,
{
    type ReadParameter = ();
    type ReadHandle = PatchSetReader<'a>;

    type WriteParameter = ();
    type WriteHandle = PatchSetWriter<'a, 'b>;

    fn read(body: Space<'a>, _: ()) -> Option<PatchSetReader> {
        println!("Property read");
        Some(PatchSetReader { space: body })
    }

    fn init(frame: FramedMutSpace<'a, 'b>, _: ()) -> Option<PatchSetWriter<'a, 'b>> {
        Some(PatchSetWriter { frame })
    }
}



pub struct PatchSetReader<'a> {
    space: Space<'a>,
}

impl<'a> PatchSetReader<'a> {
}

pub struct PatchSetWriter<'a, 'b> {
    frame: FramedMutSpace<'a, 'b>
}


impl<'a, 'b> Atom<'a, 'b> for PatchProperty
where
    'a: 'b,
{
    type ReadParameter = ();
    type ReadHandle = PatchPropertyReader<'a>;

    type ReadParameter = ();
    type ReadHandle = PatchPropertyWriter<'b>;

    fn read(body: Space<'a>, _: ()) -> Option<SequenceIterator> {
        Some(PatchPropertyReader::new(body))
    }

    fn init(
        mut frame: FramedMutSpace<'a, 'b>,
        unit: TimeStampURID,
    ) -> Option<SequenceWriter<'a, 'b>> {
        None
    }
}

impl<'a, 'b> Atom<'a, 'b> for PatchValue
where
    'a: 'b,
{
    type ReadParameter = ();
    type ReadHandle = PatchValueReader<'a>;

    type ReadParameter = ();
    type ReadHandle = PatchValueWriter<'b>;

    fn read(body: Space<'a>, _: ()) -> Option<SequenceIterator> {
        Some(PatchValueReader::new(body))
    }

    fn init(
        mut frame: FramedMutSpace<'a, 'b>,
        unit: TimeStampURID,
    ) -> Option<SequenceWriter<'a, 'b>> {
        None
    }
}
*/
