FROM --platform=arm64 busybox
WORKDIR /var/task/
COPY app.mjs ./
ADD https://github.com/awslabs/llrt/releases/download/latest/llrt-container-arm64 /usr/bin/llrt
RUN chmod +x /usr/bin/llrt

ENV LAMBDA_HANDLER "app.handler"

CMD [ "llrt" ]
