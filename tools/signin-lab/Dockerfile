FROM python:3.13-slim
RUN pip install --no-cache-dir "pyjwt[crypto]==2.10.1"
WORKDIR /app
COPY server.py index.html ./
USER nobody
EXPOSE 8099
CMD ["python", "-u", "server.py"]
