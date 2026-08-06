#[path = "protocol/iapplicable.rs"]
pub mod iapplicable;
#[path = "protocol/iassoc.rs"]
pub mod iassoc;
#[path = "protocol/icas.rs"]
pub mod icas;
#[path = "protocol/iclose.rs"]
pub mod iclose;
#[path = "protocol/icoll.rs"]
pub mod icoll;
#[path = "protocol/icomponent.rs"]
pub mod icomponent;
#[path = "protocol/iconj.rs"]
pub mod iconj;
#[path = "protocol/icons.rs"]
pub mod icons;
#[path = "protocol/icontext.rs"]
pub mod icontext;
#[path = "protocol/icontextlifecycle.rs"]
pub mod icontextlifecycle;
#[path = "protocol/icoroutine.rs"]
pub mod icoroutine;
#[path = "protocol/icount.rs"]
pub mod icount;
#[path = "protocol/ideref.rs"]
pub mod ideref;
#[path = "protocol/idereftimeout.rs"]
pub mod idereftimeout;
#[path = "protocol/idisplay.rs"]
pub mod idisplay;
#[path = "protocol/idissoc.rs"]
pub mod idissoc;
#[path = "protocol/iempty.rs"]
pub mod iempty;
#[path = "protocol/iequality.rs"]
pub mod iequality;
#[path = "protocol/iexinfo.rs"]
pub mod iexinfo;
#[path = "protocol/ifind.rs"]
pub mod ifind;
#[path = "protocol/ifn.rs"]
pub mod ifn;
#[path = "protocol/ihash.rs"]
pub mod ihash;
#[path = "protocol/ihashcached.rs"]
pub mod ihashcached;
#[path = "protocol/iindexed.rs"]
pub mod iindexed;
#[path = "protocol/iindexedkv.rs"]
pub mod iindexedkv;
#[path = "protocol/iinvokein.rs"]
pub mod iinvokein;
#[path = "protocol/iiter.rs"]
pub mod iiter;
#[path = "protocol/iiterator.rs"]
pub mod iiterator;
#[path = "protocol/ilookup.rs"]
pub mod ilookup;
#[path = "protocol/imetadata.rs"]
pub mod imetadata;
#[path = "protocol/imutable.rs"]
pub mod imutable;
#[path = "protocol/inamespaced.rs"]
pub mod inamespaced;
#[path = "protocol/inth.rs"]
pub mod inth;
#[path = "protocol/iobjtype.rs"]
pub mod iobjtype;
#[path = "protocol/iofn.rs"]
pub mod iofn;
#[path = "protocol/ipair.rs"]
pub mod ipair;
#[path = "protocol/ipeekfirst.rs"]
pub mod ipeekfirst;
#[path = "protocol/ipeeklast.rs"]
pub mod ipeeklast;
#[path = "protocol/ipersistent.rs"]
pub mod ipersistent;
#[path = "protocol/ipointer.rs"]
pub mod ipointer;
#[path = "protocol/ipopfirst.rs"]
pub mod ipopfirst;
#[path = "protocol/ipoplast.rs"]
pub mod ipoplast;
#[path = "protocol/ipromise.rs"]
pub mod ipromise;
#[path = "protocol/ipushfirst.rs"]
pub mod ipushfirst;
#[path = "protocol/ipushlast.rs"]
pub mod ipushlast;
#[path = "protocol/irealize.rs"]
pub mod irealize;
#[path = "protocol/ireduce.rs"]
pub mod ireduce;
#[path = "protocol/ireset.rs"]
pub mod ireset;
#[path = "protocol/ispace.rs"]
pub mod ispace;
#[path = "protocol/itomutable.rs"]
pub mod itomutable;
#[path = "protocol/itopersistent.rs"]
pub mod itopersistent;
#[path = "protocol/iwatch.rs"]
pub mod iwatch;

pub use iapplicable::IApplicable;
pub use iassoc::IAssoc;
pub use icas::ICas;
pub use iclose::IClose;
pub use icoll::IColl;
pub use icomponent::IComponent;
pub use iconj::IConj;
pub use icons::ICons;
pub use icontext::IContext;
pub use icontextlifecycle::IContextLifeCycle;
pub use icoroutine::ICoroutine;
pub use icount::ICount;
pub use ideref::IDeref;
pub use idereftimeout::IDerefTimeout;
pub use idisplay::IDisplay;
pub use idissoc::IDissoc;
pub use iempty::IEmpty;
pub use iequality::IEquality;
pub use iexinfo::IExInfo;
pub use ifind::IFind;
pub use ifn::IFn;
pub use ihash::{HashType, IHash};
pub use ihashcached::IHashCached;
pub use iindexed::IIndexed;
pub use iindexedkv::IIndexedKV;
pub use iinvokein::IInvokeIn;
pub use iiter::IIter;
pub use iiterator::IIterator;
pub use ilookup::ILookup;
pub use imetadata::{IMetadata, MetaType};
pub use imutable::IMutable;
pub use inamespaced::INamespaced;
pub use inth::INth;
pub use iobjtype::{IObjType, ObjType};
pub use iofn::IOFn;
pub use ipair::IPair;
pub use ipeekfirst::IPeekFirst;
pub use ipeeklast::IPeekLast;
pub use ipersistent::IPersistent;
pub use ipointer::IPointer;
pub use ipopfirst::IPopFirst;
pub use ipoplast::IPopLast;
pub use ipromise::IPromise;
pub use ipushfirst::IPushFirst;
pub use ipushlast::IPushLast;
pub use irealize::IRealize;
pub use ireduce::IReduce;
pub use ireset::IReset;
pub use ispace::ISpace;
pub use itomutable::IToMutable;
pub use itopersistent::IToPersistent;
pub use iwatch::IWatch;

#[cfg(test)]
mod tests {
    use super::{IFind, IObjType, ObjType};
    use crate::lang::data::{Cons, List, Queue, Seq, Tuple, Vector};

    struct Entries(Vec<(u8, Option<u8>)>);

    impl IFind<u8> for Entries {
        type Output = (u8, Option<u8>);

        fn find(&self, key: &u8) -> Option<Self::Output> {
            self.0
                .iter()
                .find(|(candidate, _)| candidate == key)
                .cloned()
        }
    }

    #[test]
    fn sequential_family_uses_java_protocol_category() {
        assert_eq!(List::<i32>::new().obj_type(), ObjType::Sequential);
        assert_eq!(Vector::<i32>::new().obj_type(), ObjType::Sequential);
        assert_eq!(Tuple::<i32>::Tup0.obj_type(), ObjType::Sequential);
        assert_eq!(Queue::<i32>::new().obj_type(), ObjType::Sequential);
        assert_eq!(Cons::new(1, List::new()).obj_type(), ObjType::Sequential);
        assert_eq!(Seq::new([1].into_iter()).obj_type(), ObjType::Sequential);
    }

    #[test]
    fn find_has_distinguishes_absence_from_a_nil_value() {
        let entries = Entries(vec![(1, None), (2, Some(7))]);
        assert_eq!(entries.find(&1), Some((1, None)));
        assert!(entries.has(&1));
        assert!(entries.has(&2));
        assert!(!entries.has(&3));
    }
}
