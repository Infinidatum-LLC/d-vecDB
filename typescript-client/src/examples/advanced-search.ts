/**
 * Advanced search example with filtering and HNSW parameters
 */

import { VectorDBClient, DistanceMetric } from '../index';

async function main(): Promise<void> {
  const client = new VectorDBClient({
    host: 'localhost',
    port: 8080,
  });

  try {
    // Create collection with custom HNSW parameters
    console.log('Creating collection with custom HNSW parameters...');
    const collectionName = 'advanced_search_collection';
    await client.createCollection({
      name: collectionName,
      dimension: 64,
      distanceMetric: DistanceMetric.COSINE,
      indexConfig: {
        maxConnections: 32,
        efConstruction: 400,
        efSearch: 100,
        maxLayer: 16,
      },
    });
    console.log('Collection created');

    // Insert vectors with rich metadata
    console.log('\nInserting vectors with metadata...');
    const categories = ['electronics', 'books', 'clothing', 'food'];
    const vectorsData: Array<[string, number[], Record<string, string | number>]> = [];

    for (let i = 1; i <= 200; i++) {
      const vector = Array.from({ length: 64 }, () => Math.random());
      const category = categories[i % categories.length];
      const price = Math.floor(Math.random() * 1000);

      vectorsData.push([
        `product-${i}`,
        vector,
        {
          category,
          price: price.toString(),
          name: `Product ${i}`,
          inStock: i % 3 === 0 ? 'true' : 'false',
        },
      ]);
    }

    await client.batchInsertSimple(collectionName, vectorsData, 100);
    console.log('Vectors inserted');

    // Example 1: Basic search
    console.log('\n--- Example 1: Basic Search ---');
    const queryVector1 = Array.from({ length: 64 }, () => Math.random());
    const results1 = await client.searchSimple(collectionName, queryVector1, 5);
    console.log('Top 5 results:');
    results1.forEach((result, i) => {
      console.log(`  ${i + 1}. ${result.id} (distance: ${result.distance.toFixed(4)})`);
      console.log(`     ${JSON.stringify(result.metadata)}`);
    });

    // Example 2: Search with higher limit
    console.log('\n--- Example 2: Search with Higher Limit ---');
    const results2 = await client.searchSimple(collectionName, queryVector1, 20);
    console.log(`Found ${results2.length} results`);

    // Example 3: Search with custom efSearch (higher = more accurate, slower)
    console.log('\n--- Example 3: Search with Custom efSearch ---');
    const queryVector2 = Array.from({ length: 64 }, () => Math.random());
    const results3 = await client.searchSimple(collectionName, queryVector2, 10, 200);
    console.log('Results with efSearch=200:');
    results3.slice(0, 5).forEach((result, i) => {
      console.log(`  ${i + 1}. ${result.id} (distance: ${result.distance.toFixed(4)})`);
    });

    // Example 4: Search with metadata filter
    console.log('\n--- Example 4: Search with Metadata Filter ---');
    const queryVector3 = Array.from({ length: 64 }, () => Math.random());
    const results4 = await client.searchSimple(
      collectionName,
      queryVector3,
      10,
      undefined,
      { category: 'electronics' }
    );
    console.log('Results filtered by category=electronics:');
    results4.forEach((result, i) => {
      console.log(`  ${i + 1}. ${result.id} (distance: ${result.distance.toFixed(4)})`);
      console.log(`     Category: ${result.metadata?.category}`);
    });

    // Example 5: Multiple searches (parallelization example)
    console.log('\n--- Example 5: Parallel Searches ---');
    const start = Date.now();
    const searchPromises = [];
    for (let i = 0; i < 10; i++) {
      const queryVector = Array.from({ length: 64 }, () => Math.random());
      searchPromises.push(client.searchSimple(collectionName, queryVector, 5));
    }
    const parallelResults = await Promise.all(searchPromises);
    const elapsed = Date.now() - start;
    console.log(`Completed 10 parallel searches in ${elapsed}ms`);
    console.log(`Average: ${(elapsed / 10).toFixed(2)}ms per search`);

    // Clean up
    console.log('\nCleaning up...');
    await client.deleteCollection(collectionName);
    console.log('Collection deleted');

    console.log('\nAdvanced search example completed!');
  } catch (error) {
    console.error('Error:', error);
  } finally {
    client.close();
  }
}

if (require.main === module) {
  main().catch(console.error);
}

export { main };
