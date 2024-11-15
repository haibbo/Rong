.PHONY: all clean

# Default compiler and archiver
CC ?= clang
AR ?= ar

# Compilation flags
CFLAGS ?= -fPIC -O2 -Wall

# Directories
SRC_DIR_QJS = quickjs-ng
SRC_DIR_PATCH = patch
OUT_DIR ?= out

# Header file directories
INCLUDE_DIRS = -I$(SRC_DIR_QJS) -I$(SRC_DIR_PATCH)

# necessary source files of quickjs but except quickjs.c
QJS_FILENAMES = libregexp.c libunicode.c cutils.c libbf.c

# To append extra functions to quickjs.c
MERGE_SRC_FILES = $(SRC_DIR_QJS)/quickjs.c $(SRC_DIR_PATCH)/extra.c
NEW_QUICKJS = $(OUT_DIR)/new_quickjs.c

# Construct the full paths for the files in SRC_DIR_QJS
QJS_FILES = $(addprefix $(SRC_DIR_QJS)/, $(QJS_FILENAMES))
PATCH_FILES = $(filter-out $(SRC_DIR_PATCH)/extra.c, $(wildcard $(SRC_DIR_PATCH)/*.c))

LIB_TARGET = $(OUT_DIR)/libquickjs.a

# Default target
all: $(LIB_TARGET)

$(OUT_DIR):
	@mkdir -p $(OUT_DIR)

# append extra.c to quickjs.c
$(NEW_QUICKJS): $(MERGE_SRC_FILES) | $(OUT_DIR)
	cat $^ > $@

# all dependent object files
OBJ_FILES = $(patsubst $(SRC_DIR_QJS)/%.c,$(OUT_DIR)/%.o,$(QJS_FILES)) \
            $(patsubst $(SRC_DIR_PATCH)/%.c,$(OUT_DIR)/%.o,$(PATCH_FILES)) \
            $(OUT_DIR)/new_quickjs.o


$(OUT_DIR)/%.o: $(SRC_DIR_QJS)/%.c | $(OUT_DIR)
	$(CC) $(CFLAGS) $(INCLUDE_DIRS) -c $< -o $@

$(OUT_DIR)/%.o: $(SRC_DIR_PATCH)/%.c | $(OUT_DIR)
	$(CC) $(CFLAGS) $(INCLUDE_DIRS) -c $< -o $@

$(OUT_DIR)/new_quickjs.o: $(NEW_QUICKJS)
	$(CC) $(CFLAGS) $(INCLUDE_DIRS) -c $< -o $@

$(LIB_TARGET): $(OBJ_FILES)
	$(AR) rcs $@ $^

clean:
	@rm -rf $(OUT_DIR)
