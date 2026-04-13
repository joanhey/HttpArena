import { Elysia, status } from "elysia";
import { staticPlugin } from "@elysiajs/static";

import { SQL } from "bun";
import { Database, Statement } from "bun:sqlite";

import cluster from "cluster";
import { availableParallelism } from "os";
import { readFileSync } from "fs";

for (let i = 0; i < availableParallelism(); i++) {
	if (cluster.isPrimary) cluster.fork();
	else {
		const datasetItems: any[] = JSON.parse(
			readFileSync("/data/dataset.json", "utf8"),
		);

		let statement: Statement<any, any> | undefined;
		for (let attempt = 0; attempt < 3 && !statement; attempt++) {
			try {
				const db = new Database("/data/benchmark.db", {
					readonly: true,
				});
				db.run("PRAGMA mmap_size=268435456");
				statement = db.prepare(
					"SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ? AND ? LIMIT 50",
				);
			} catch (e) {
				console.error(`SQLite open attempt ${attempt + 1} failed:`, e);
				if (attempt < 2) Bun.sleepSync(50);
			}
		}

		const databaseURL = process.env.DATABASE_URL;
		const pg = databaseURL ? new SQL(databaseURL) : undefined;
		if (pg) await pg.connect();

		new Elysia()
			.headers({
				server: "Elysia",
			})
			.use(
				staticPlugin({
					assets: "/data/static",
					prefix: "/static",
					etag: false,
				}),
			)
			.get("/pipeline", "ok")
			.get("/baseline11", ({ query }) => {
				let sum = 0;
				for (const v of Object.values(query)) sum += +v || 0;
				return sum;
			})
			.post(
				"/baseline11",
				({ query, body }) => {
					let total = 0;
					for (const v of Object.values(query)) total += +v || 0;

					const n = +(body as string);
					if (!isNaN(n)) total += n;

					return total;
				},
				{
					parse: "text",
				},
			)
			.get("/baseline2", ({ query }) => {
				let sum = 0;
				for (const v of Object.values(query)) sum += +v || 0;
				return sum;
			})
			.get("/json/:count", ({ params, query, headers, set }) => {
				const count = Math.max(
					0,
					Math.min(+params.count || 0, datasetItems.length),
				);
				const m = query.m ? +query.m || 1 : 1;

				const result = {
					count,
					items: datasetItems.slice(0, count).map((d: any) => ({
						id: d.id,
						name: d.name,
						category: d.category,
						price: d.price,
						quantity: d.quantity,
						active: d.active,
						tags: d.tags,
						rating: d.rating,
						total: d.price * d.quantity * m,
					})),
				};

				const encoding = headers["accept-encoding"];
				if (encoding) {
					const index = encoding.indexOf(",");
					const type =
						index === -1 ? encoding : encoding.slice(0, index);

					set.headers["content-type"] = "application/json";
					if (type === "gzip") {
						set.headers["content-encoding"] = "gzip";
						return Bun.gzipSync(JSON.stringify(result));
					} else if (encoding === "br") {
						set.headers["content-encoding"] = "br";
						return Bun.deflateSync(JSON.stringify(result));
					} else if (encoding === "deflate") {
						set.headers["content-encoding"] = "deflate";
						return Bun.deflateSync(JSON.stringify(result));
					}
				}

				return result;
			})
			.get("/db", ({ query }) => {
				if (!statement) return { items: [], count: 0 };

				const min = +query.min || 10;
				const max = +query.max || 50;
				const rows = statement.all(min, max);

				return {
					count: rows.length,
					items: rows.map((r: any) => ({
						id: r.id,
						name: r.name,
						category: r.category,
						price: r.price,
						quantity: r.quantity,
						active: r.active === 1,
						tags: JSON.parse(r.tags),
						rating: {
							score: r.rating_score,
							count: r.rating_count,
						},
					})),
				};
			})
			.get(
				"/async-db",
				async ({ query }) => {
					if (!pg) return { items: [], count: 0 };

					const min = +query.min || 10;
					const max = +query.max || 50;
					const limit = Math.max(1, Math.min(+query.limit || 50, 50));

					const result =
						await pg`SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ${min} AND ${max} LIMIT ${limit}`;

					return {
						count: result.rows.length,
						items: result.rows.map((r: any) => ({
							id: r.id,
							name: r.name,
							category: r.category,
							price: r.price,
							quantity: r.quantity,
							active: r.active,
							tags: r.tags,
							rating: {
								score: r.rating_score,
								count: r.rating_count,
							},
						})),
					};
				},
				{
					error: () => ({ items: [], count: 0 }),
				},
			)
			.post(
				"/upload",
				({ request: { body } }) =>
					(body as any as ArrayBuffer).byteLength,
				{
					parse: "arrayBuffer",
				},
			)
			.onError(({ code }) => {
				if (code === "NOT_FOUND") return status(404);
			})
			.listen(8080);
	}
}
