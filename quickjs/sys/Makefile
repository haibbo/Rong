.PHONY: all clean

# Default compiler and archiver
CC ?= clang
AR ?= ar

# it's from build.rs
PROFILE ?= debug

# Compilation flags
CFLAGS ?= -fPIC  -Wall

ifeq ($(PROFILE), release)
    CFLAGS += -O2 -DNDEBUG
else
    CFLAGS += -g -O0
endif


# Directories
SRC_DIR_QJS = quickjs-ng
SRC_DIR_PATCH = patch
OUT_DIR ?= out

# Header file directories
INCLUDE_DIRS = -I$(SRC_DIR_QJS) -I$(SRC_DIR_PATCH)

# Necessary source files of quickjs (except quickjs.c)
QJS_FILENAMES = libregexp.c libunicode.c cutils.c dtoa.c

# Patch source files
PATCH_FILENAMES = qjs.c inline.c

# File name of merged quickjs.c
NEW_QUICKJS=new_quickjs.c

LIB_TARGET = $(OUT_DIR)/libquickjs.a

# Default target
all: $(LIB_TARGET)
example: async class


# Create output directory if it doesn't exist
$(OUT_DIR):
	@mkdir -p $(OUT_DIR)
	@mkdir -p $(OUT_DIR)/example

# Copy necessary source files from quickjs-ng to out directory
$(foreach file, $(QJS_FILENAMES), $(eval $(OUT_DIR)/$(file): $(SRC_DIR_QJS)/$(file) | $(OUT_DIR) ; cp $$< $$@))

# Copy necessary source files from patch to out directory
$(foreach file, $(PATCH_FILENAMES), $(eval $(OUT_DIR)/$(file): $(SRC_DIR_PATCH)/$(file) | $(OUT_DIR) ; cp $$< $$@))

# Merge quickjs.c from quickjs-ng and extra.c from patch into new_quickjs.c
$(OUT_DIR)/$(NEW_QUICKJS): $(SRC_DIR_QJS)/quickjs.c $(SRC_DIR_PATCH)/extra.c | $(OUT_DIR)
	cat $^ > $@

# Compile all source files in out directory to object files
$(OUT_DIR)/%.o: $(OUT_DIR)/%.c
	$(CC) $(CFLAGS) $(INCLUDE_DIRS) -c $< -o $@

# Build the library from object files
$(LIB_TARGET): $(addprefix $(OUT_DIR)/, $(patsubst %.c, %.o, $(QJS_FILENAMES) $(PATCH_FILENAMES) $(NEW_QUICKJS)))
	$(AR) rcs $@ $^

# Clean up the output directory
clean:
	@rm -rf $(OUT_DIR)

$(OUT_DIR)/example/%.o: $(SRC_DIR_PATCH)/example/%.c
	$(CC) $(CFLAGS) $(INCLUDE_DIRS) -c $< -o $@

class: $(LIB_TARGET) $(OUT_DIR)/example/class.o
	$(CC) $^ -o $(OUT_DIR)/$@

async: $(LIB_TARGET) $(OUT_DIR)/example/async.o
	$(CC) $^ -luv -o $(OUT_DIR)/$@
