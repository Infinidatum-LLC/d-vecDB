/**
 * Basic usage example for d-vecDB TypeScript client
 */

import { VectorDBClient, DistanceMetric } from '../index';

async function main(): Promise<void> {
  // Create client
  const client = new VectorDBClient({
    host: 'localhost',
    port: 8080,
    timeout: 30000,
  });

  try {
    // 1. Test connection
    console.log('Testing connection...');
    const isAlive = await client.ping();
    console.log(`Server is ${isAlive ? 'reachable' : 'unreachable'}`);

    if (!isAlive) {
      console.error('Server is not reachable. Exiting.');
      return;
    }

    // 2. Create a collection
    console.log('\nCreating collection...');
    const collectionName = 'example_collection';
    await client.createCollectionSimple(collectionName, 128, DistanceMetric.COSINE);
    console.log(`Collection '${collectionName}' created`);

    // 3. Insert a single vector
    console.log('\nInserting single vector...');
    const vector1 = Array.from({ length: 128 }, () => Math.random());
    await client.insertSimple(collectionName, 'vec-1', vector1, {
      label: 'example',
      category: 'test',
    });
    console.log('Vector inserted');

    // 4. Batch insert vectors
    console.log('\nBatch inserting vectors...');
    const vectorsData: Array<[string, number[], Record<string, string>]> = [];
    for (let i = 2; i <= 100; i++) {
      const vector = Array.from({ length: 128 }, () => Math.random());
      vectorsData.push([`vec-${i}`, vector, { label: `item-${i}` }]);
    }
    await client.batchInsertSimple(collectionName, vectorsData, 50);
    console.log('Batch insert complete');

    // 5. Get collection stats
    console.log('\nGetting collection stats...');
    const stats = await client.getCollectionStats(collectionName);
    console.log(`Collection stats:
      - Vectors: ${stats.vectorCount}
      - Dimension: ${stats.dimension}
      - Index size: ${stats.indexSize} bytes
      - Memory usage: ${stats.memoryUsage} bytes`);

    // 6. Search for similar vectors
    console.log('\nSearching for similar vectors...');
    const queryVector = Array.from({ length: 128 }, () => Math.random());
    const results = await client.searchSimple(collectionName, queryVector, 5);
    console.log(`Found ${results.length} results:`);
    results.forEach((result, i) => {
      console.log(`  ${i + 1}. ID: ${result.id}, Distance: ${result.distance.toFixed(4)}`);
      if (result.metadata) {
        console.log(`     Metadata: ${JSON.stringify(result.metadata)}`);
      }
    });

    // 7. Get a specific vector
    console.log('\nRetrieving vector...');
    const retrievedVector = await client.getVector(collectionName, 'vec-1');
    console.log(`Retrieved vector: ${retrievedVector.id}`);
    console.log(`  Dimension: ${retrievedVector.data.length}`);
    console.log(`  Metadata: ${JSON.stringify(retrievedVector.metadata)}`);

    // 8. Update a vector
    console.log('\nUpdating vector...');
    const newVector = Array.from({ length: 128 }, () => Math.random() * 2);
    await client.updateVector(collectionName, {
      id: 'vec-1',
      data: newVector,
      metadata: { label: 'updated', version: '2' },
    });
    console.log('Vector updated');

    // 9. Delete a vector
    console.log('\nDeleting vector...');
    await client.deleteVector(collectionName, 'vec-2');
    console.log('Vector deleted');

    // 10. Get server stats
    console.log('\nGetting server stats...');
    const serverStats = await client.getServerStats();
    console.log(`Server stats:
      - Total vectors: ${serverStats.totalVectors}
      - Total collections: ${serverStats.totalCollections}
      - Memory usage: ${serverStats.memoryUsage} bytes
      - Disk usage: ${serverStats.diskUsage} bytes
      - Uptime: ${serverStats.uptimeSeconds} seconds`);

    // 11. List all collections
    console.log('\nListing collections...');
    const collections = await client.listCollections();
    console.log(`Found ${collections.collections.length} collections:`);
    collections.collections.forEach(col => {
      console.log(`  - ${col.name} (${col.dimension}D, ${col.distanceMetric})`);
    });

    // 12. Clean up - delete collection
    console.log('\nCleaning up...');
    await client.deleteCollection(collectionName);
    console.log('Collection deleted');

    console.log('\nExample completed successfully!');
  } catch (error) {
    console.error('Error:', error);
  } finally {
    client.close();
  }
}

// Run the example
if (require.main === module) {
  main().catch(console.error);
}

export { main };
