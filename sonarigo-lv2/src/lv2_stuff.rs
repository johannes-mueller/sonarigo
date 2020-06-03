

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

#[uri("http://lv2plug.in/ns/ext/atom#Path")]
pub struct AtomPath;

impl<'a, 'b> Atom<'a, 'b> for AtomPath
where 'a: 'b,
{
    type ReadParameter = ();
    type ReadHandle = &'a str;

    type WriteParameter = ();
    type WriteHandle = AtomPathWriter<'a, 'b>;

    fn read(body: Space<'a>, _: ()) -> Option<&'a str> {
        body.data()
            .and_then(|data| std::str::from_utf8(data).ok())
            .map(|path| path.trim_matches(char::from(0)))
    }

    fn init(frame: FramedMutSpace<'a, 'b>, _: ()) -> Option<AtomPathWriter<'a, 'b>> {
        Some(AtomPathWriter { frame })
    }
}

pub struct AtomPathWriter<'a, 'b> {
    frame: FramedMutSpace<'a, 'b>
}

impl<'a, 'b> AtomPathWriter<'a, 'b> {
    pub fn append(&mut self, string: &str) -> Option<&mut str> {
        let data = string.as_bytes();
        let space = self.frame.write_raw(data, false)?;
        unsafe { Some(std::str::from_utf8_unchecked_mut(space)) }
    }
}
