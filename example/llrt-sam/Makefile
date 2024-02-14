build-LlrtLayer:
	curl -s https://api.github.com/repos/awslabs/llrt/releases/latest | grep "llrt-lambda-x64.zip" | cut -d : -f 2,3 | tail -1 | xargs curl -o llrt.zip -L
	unzip llrt.zip
	mv bootstrap $(ARTIFACTS_DIR)
