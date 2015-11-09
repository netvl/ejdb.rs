use ejdb_sys;

use super::Collection;
use Result;

impl<'db> Collection<'db> {
    /// Starts a transaction, returning a guard object for it.
    ///
    /// This method can be used to start transactions over an EJDB collection. Transactions
    /// are controlled with their guard objects, which rely on RAII pattern to abort or
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
    pub fn begin_transaction(&self) -> Result<Transaction> { Transaction::new(self) }

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
        let mut result = 0;
        if unsafe { ejdb_sys::ejdbtranstatus(self.coll, &mut result) != 0 } {
            Ok(result != 0)
        } else {
            self.db.last_error("error getting transaction status")
        }
    }

}

pub struct Transaction<'coll, 'db: 'coll> {
    coll: &'coll Collection<'db>,
    commit: bool,
    finished: bool
}

impl<'coll, 'db> Drop for Transaction<'coll, 'db> {
    fn drop(&mut self) {
        let _ = self.finish_mut();  // ignore the result
    }
}

impl<'coll, 'db> Transaction<'coll, 'db> {
    fn new(coll: &'coll Collection<'db>) -> Result<Transaction<'coll, 'db>> {
        if unsafe { ejdb_sys::ejdbtranbegin(coll.coll) != 0 } {
            coll.db.last_error("error opening transaction")
        } else {
            Ok(Transaction { coll: coll, commit: false, finished: false })
        }
    }

    #[inline]
    pub fn will_commit(&self) -> bool { self.commit }

    #[inline]
    pub fn will_abort(&self) -> bool { !self.commit }

    #[inline]
    pub fn set_commit(&mut self) { self.commit = true; }

    #[inline]
    pub fn set_abort(&mut self) { self.commit = false; }

    #[inline]
    pub fn finish(mut self) -> Result<()> { self.finish_mut() }

    #[inline]
    pub fn commit(mut self) -> Result<()> { self.commit_mut() }

    #[inline]
    pub fn abort(mut self) -> Result<()> { self.abort_mut() }

    fn finish_mut(&mut self) -> Result<()> {
        if self.finished { Ok(()) }
        else { if self.commit { self.commit_mut() } else { self.abort_mut() } }
    }

    fn commit_mut(&mut self) -> Result<()> {
        self.finished = true;
        if unsafe { ejdb_sys::ejdbtrancommit(self.coll.coll) != 0 } { Ok(()) }
        else { self.coll.db.last_error("error commiting transaction") }
    }

    fn abort_mut(&mut self) -> Result<()> {
        self.finished = true;
        if unsafe { ejdb_sys::ejdbtranabort(self.coll.coll) != 0 } { Ok(()) }
        else { self.coll.db.last_error("error aborting transaction") }
    }
}
