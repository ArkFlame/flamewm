#include "config.h"
#include "ref.h"

// an inaccessible class
class null_ref {
public:
    // pointer to null_ref
    static null_ref a_null_storage;
    static null_ref* a_null_ptr;
private:
    // cannot instantiate
    null_ref() { }
    // cannot copy
    null_ref(const null_ref&);
};

// Keep the sentinel object alive for the lifetime of the process.
null_ref null_ref::a_null_storage;
null_ref* null_ref::a_null_ptr(&null_ref::a_null_storage);

// null references the sentinel object without exposing its type.
null_ref& null(*null_ref::a_null_ptr);

// reference count became zero
void refcounted::__destroy() {
    delete this;
}

// vim: set sw=4 ts=4 et:
