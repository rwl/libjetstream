PROFILE = dev
INC = ..
LIBDIR = ../target/debug

all: example

example: main.cpp $(INC)/jetstream.h $(LIBDIR)/libcjetstream.so
	$(CXX) -o $@ -g -I$(INC) -L$(LIBDIR) main.cpp -lcjetstream
	#$(CXX) -o $@ -I$(INC) main.cpp $(LIBDIR)/libcjetstream.so

$(LIBDIR)/libcjetstream.so:
	cd .. && cargo build --profile $(PROFILE)

check: example
	LD_LIBRARY_PATH=$(LIBDIR) ./example

clean:
	rm -f ./example
