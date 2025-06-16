TARGET = afd
RUSTC = rustc
RUSTFLAGS = --edition 2021 -C opt-level=2 -C panic=abort -C link-arg=-nostartfiles -C link-arg=-static

.PHONY: all clean run

all: $(TARGET)

$(TARGET): afd.rs
	$(RUSTC) $(RUSTFLAGS) -o $(TARGET) afd.rs

clean:
	rm -f $(TARGET)

run: $(TARGET)
	./$(TARGET)
