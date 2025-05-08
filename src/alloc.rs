unsafe extern "C" {
    pub fn qjs_custom_calloc(count: usize, size: usize) -> *mut ();
    pub fn qjs_custom_malloc(size: usize) -> *mut ();
    pub fn qjs_custom_free(ptr: *mut ());
    pub fn qjs_custom_realloc(ptr: *mut (), size: usize) -> *mut ();
    pub fn qjs_custom_malloc_usable_size(ptr: *mut ()) -> usize;
}
