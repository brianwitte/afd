TARGET = forth
RUSTC = rustc
RUSTFLAGS = --edition 2021 -C opt-level=2 -C panic=abort -C link-arg=-nostartfiles -C link-arg=-static

.PHONY: all clean run

all: $(TARGET)

$(TARGET): main.rs
	$(RUSTC) $(RUSTFLAGS) -o $(TARGET) main.rs

clean:
	rm -f $(TARGET)

run: $(TARGET)
	./$(TARGET)
