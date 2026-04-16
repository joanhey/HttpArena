FROM python:3.13-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE 8080
ENV WRK_COUNT=64
CMD ["fastapi", "run", "app.py", "--port", "8080", "--workers", "${WRK_COUNT}"]
