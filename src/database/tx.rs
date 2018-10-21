use ejdb_sys;

use super::Collection;
use Result;

impl<'db> Collection<'db> {
    /// Starts a transaction, returning a guard object for it.
    ///
    /// This method can be used to start transactions over an EJDB collection. Transactions
    /// are controlled with their guard objects, which rely on the RAII pattern to abort or
    /// commit transactions when appropriate.
    ///
    /// # Failures
    ///
    /// Returns an error if the corresponding EJDB operation can't be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let tx = coll.begin_transaction().unwrap();
    /// // transaction is now active until `tx` goes out of scope or otherwise consumed
    /// ```
    #[inline]
    pub fn begin_transaction(&self) -> Result<Transaction> {
        Transaction::new(self)
    }

    /// Checks whether there is an active transaction on this collection.
    ///
    /// # Failures
    ///
    /// Returns an error if the corresponding EJDB operation can't be completed successfully.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ejdb::Database;
    /// let db = Database::open("/path/to/db").unwrap();
    /// let coll = db.collection("some_collection").unwrap();
    /// let tx = coll.begin_transaction().unwrap();
    /// assert!(coll.transaction_active().unwrap());
    /// ```
    pub fn transaction_active(&self) -> Result<bool> {
        let mut result = false;
        if unsafe { ejdb_sys::ejdbtranstatus(self.coll, &mut result) } {
            Ok(result)
        } else {
            self.db.last_error("error getting transaction status")
        }
    }
}

/// Represents an active transaction.
///
/// This structure is a transaction guard for an EJDB collection. It employs the RAII pattern:
/// a value of this structure is returned when a transaction is started and when this value
/// is dropped, the transaction is committed or aborted.
///
/// By default when a transaction goes out of scope, it is aborted. This is done because
/// of the way how errors are handled in Rust: if you interleave working with an EJDB collection
/// with other, potentially failing operations, then it is possible for an error to cause
/// an early return, dropping the transaction in progress. In this case aborting the transaction
/// is usually the most natural option.
///
/// However, it is possible to change the default behavior with `set_commit()/set_abort()` methods,
/// and it is also possible to commit or abort the transaction with `commit()`/`abort()` methods.
/// `finish()` method is essentially equivalent to dropping the transaction guard, except that
/// it returns a value which may contain an error (for regular drops any errors are ignored).
/// `will_commit()`/`will_abort()` methods return `true` if their respective action will be taken
/// upon finishing.
///
/// In EJDB transactions can only span one collection, therefore transactions created from a
/// collection has a direct lifetime dependency on it.
///
/// See `Collection::begin_transaction()` documentation for examples.
pub struct Transaction<'coll, 'db: 'coll> {
    coll: &'coll Collection<'db>,
    commit: bool,
    finished: bool,
}

impl<'coll, 'db> Drop for Transaction<'coll, 'db> {
    fn drop(&mut self) {
        let _ = self.finish_mut(); // ignore the result
    }
}

impl<'coll, 'db> Transaction<'coll, 'db> {
    fn new(coll: &'coll Collection<'db>) -> Result<Transaction<'coll, 'db>> {
        if unsafe { ejdb_sys::ejdbtranbegin(coll.coll) } {
            coll.db.last_error("error opening transaction")
        } else {
            Ok(Transaction {
                coll: coll,
                commit: false,
                finished: false,
            })
        }
    }

    /// Checks whether this transaction will be committed upon drop.
    ///
    /// Returns `true` if this transaction will be committed when dropped or when `finish()`
    /// method is called.
    #[inline]
    pub fn will_commit(&self) -> bool {
        self.commit
    }

    /// Checks whether this transaction will be aborted upon drop.
    ///
    /// Returns `true` if this transaction will be aborted when dropped or when `finish()`
    /// method is called.
    #[inline]
    pub fn will_abort(&self) -> bool {
        !self.commit
    }

    /// Makes this transaction commit when dropped.
    #[inline]
    pub fn set_commit(&mut self) {
        self.commit = true;
    }

    /// Makes this transaction abort when dropped.
    #[inline]
    pub fn set_abort(&mut self) {
        self.commit = false;
    }

    /// Aborts or commits the transaction depending on the finish mode.
    ///
    /// The mode can be changed with `set_commit()` and `set_abort()` methods.
    #[inline]
    pub fn finish(mut self) -> Result<()> {
        self.finish_mut()
    }

    /// Attempts to commit this transaction.
    #[inline]
    pub fn commit(mut self) -> Result<()> {
        self.commit_mut()
    }

    /// Attempts to abort this transaction.
    #[inline]
    pub fn abort(mut self) -> Result<()> {
        self.abort_mut()
    }

    fn finish_mut(&mut self) -> Result<()> {
        if self.finished {
            Ok(())
        } else {
            if self.commit {
                self.commit_mut()
            } else {
                self.abort_mut()
            }
        }
    }

    fn commit_mut(&mut self) -> Result<()> {
        self.finished = true;
        if unsafe { ejdb_sys::ejdbtrancommit(self.coll.coll) } {
            Ok(())
        } else {
            self.coll.db.last_error("error commiting transaction")
        }
    }

    fn abort_mut(&mut self) -> Result<()> {
        self.finished = true;
        if unsafe { ejdb_sys::ejdbtranabort(self.coll.coll) } {
            Ok(())
        } else {
            self.coll.db.last_error("error aborting transaction")
        }
    }
}
