import "dotenv/config";

import postgres from 'postgres';
import { faker } from '@faker-js/faker';
import * as crypto from "node:crypto";
import { Client } from 'pg';

async function main() {

	let databaseUrl = process.env.DATABASE_URL;

	if (databaseUrl === undefined) {
		throw new Error("DATABASE_URL is undefined");
	}

	console.log(databaseUrl)

	let client = new Client(databaseUrl)
	await client.connect();

	console.log("Delete current tasks");
	await client.query("delete from chang.tasks", []);

	let kinds = getStringArray();
	let queues = getStringArray(20);

	let batchSize = 1000;
	for (let i = 0; i < batchSize; i = i + 1) {
		let tasks = generateTasks(kinds, queues, 750);
		console.log(`Insert Batch: ${i + 1}/${batchSize}`);

		await client.query(`
			insert into chang.tasks(max_attempts, scheduled_at, priority, args, attempted_by, kind, queue, tags, depends_on, dependend_id)
			select * 
			  from jsonb_to_recordset($1) as x(
					max_attempts smallint
				  , scheduled_at timestamptz
				  , priority smallint
				  , args jsonb
				  , attempted_by text[]
				  , kind text
				  , queue text
				  , tags text[]
				  , depends_on uuid
				  , dependend_id uuid
				)
		`, [JSON.stringify(tasks)]);
	}
}

main()
	.then(() => process.exit(0))
	.catch(err => {
		console.log("Failed to run script: ", err);
		process.exit(1);
	});


function getStringArray(size: number = 100) {
	let result = [];

	for (let i = 0; i < size; i = i + 1) {
		result.push(faker.string.alpha({ length: { min: 5, max: 50 } }));
	}

	return result;
}

type NewTask = {
	id: string,
	max_attempts: number,
	scheduled_at: Date,
	priority: number,
	args: unknown,
	attempted_by: Array<string>,
	kind: string,
	queue: string,
	tags: Array<string>,
	depends_on: string | null,
	dependend_id: string | null,
}

function generateTasks(
	kinds: Array<string>,
	queues: Array<string>,
	batchSize: number
) {
	let batch: Array<NewTask> = [];

	for (let i = 0; i < batchSize; i = i + 1) {
		batch.push({
			id: crypto.randomUUID(),
			max_attempts: getRandomBetween(3, 5),
			scheduled_at: faker.date.anytime(),
			priority: getRandomBetween(1, 6),
			args: batch[i - 1],
			attempted_by: [],
			kind: kinds[getRandomBetween(0, kinds.length - 1)],
			queue: queues[getRandomBetween(0, queues.length - 1)],
			tags: [],
			depends_on: (batch.length > 0 && getRandomBetween(0, 5) > 3) ? batch[getRandomBetween(0, batch.length - 1)].id : null,
			dependend_id: (batch.length > 0 && getRandomBetween(0, 5) > 3) ? batch[getRandomBetween(0, batch.length - 1)].id : null,
		});
	}

	return batch;
}

function getRandomBetween(min: number, max: number): number {
    return Math.floor(Math.random() * (max - min + 1) + min);
}