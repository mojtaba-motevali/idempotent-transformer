'use client';

import { useContext, useRef, useState } from 'react';
import styles from './page.module.css';
// import { IdempotentProviderContext } from '@/lib/context-api';

const uploadPhoto = async (file: File): Promise<string> => {
  return new Promise((resolve) => {
    setTimeout(() => resolve(file.name), 1000);
  });
};

const editProfile = async (name: string): Promise<string> => {
  return new Promise((resolve) => {
    setTimeout(() => resolve(name), 500);
  });
};

const generateRandomId = () => {
  return Math.random().toString(36).substring(2, 15);
};

export default function Home() {
  const pageRef = useRef<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [file, setFile] = useState<File | null>(null);
  const [photoResult, setPhotoResult] = useState<string | null>(null);
  const [profileResult, setProfileResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [name, setName] = useState('');

  // const idempotentTransformer = useContext(IdempotentProviderContext);
  if (pageRef.current === null) {
    pageRef.current = generateRandomId();
  }
  const handleEditProfile = async () => {
    setLoading(true);

    // const idempotent = await idempotentTransformer!.makeIdempotent(pageRef.current!, {
    //   uploadPhoto,
    //   editProfile,
    // });

    const result = await uploadPhoto(file!);
    setPhotoResult(result);

    const result2 = await editProfile(name);
    setProfileResult(result2);

    // const result = await idempotent.uploadPhoto(file!);

    // setPhotoResult(result);
    // const result2 = await idempotent.editProfile(name);
    // setProfileResult(result2);

    setLoading(false);
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFile(e.target.files?.[0] || null);
  };

  return (
    <div className={styles.container}>
      <div className={styles.historySection}>
        <h2>User Profile</h2>
        {history.length === 0 && <p className={styles.empty}>No idempotent calls yet.</p>}
        <div className={styles.historyList}></div>
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          handleEditProfile();
        }}
        style={{ display: 'flex', flexDirection: 'column', gap: '16px', maxWidth: 400 }}
      >
        <div className={styles.fileInputContainer}>
          <button
            className={styles.fileInputButton}
            type="button"
            onClick={() => fileInputRef.current?.click()}
          >
            Upload Photo
          </button>
          <input
            ref={fileInputRef}
            type="file"
            onChange={handleFileChange}
            style={{ display: 'none' }}
          />
          {file && <span className={styles.fileName}>{file.name}</span>}
        </div>
        <input
          type="text"
          placeholder="Enter your name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className={styles.nameInput}
        />
        <button className={styles.ctaButton} type="submit" disabled={loading || !file}>
          {loading ? 'Processing...' : 'Submit'}
        </button>
      </form>
      <p>
        <span className={styles.label}>Upload photo result:</span>{' '}
        <span className={styles.result}>{photoResult}</span>
      </p>
      <p>
        <span className={styles.label}>Edit profile result:</span>{' '}
        <span className={styles.result}>{profileResult}</span>
      </p>
    </div>
  );
}
