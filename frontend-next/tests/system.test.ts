import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isSystemUserId } from '../src/lib/admin/audit/system.ts';

test('system users have short numbers as ids', () => {
    for (const id of ['101', '102', '1', '99999999']) assert.equal(isSystemUserId(id), true, id);
    for (const id of ['', 'system', '10a', '123456789', 'a1b2c3d4e5f6g7h8i9j0k1l2', ' 101', undefined, null]) {
        assert.equal(isSystemUserId(id as string), false, String(id));
    }
});
