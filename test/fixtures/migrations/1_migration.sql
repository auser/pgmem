-- CreateTable
CREATE TABLE "User" (
    "id" TEXT NOT NULL,
    "name" TEXT,
    "email" TEXT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,
    "passwordHash" TEXT NOT NULL,
    "verifyToken" TEXT,
    "phone" TEXT,
    "avatar" TEXT,
    "defaultTenantId" TEXT,

    CONSTRAINT "User_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "AdminUser" (
    "userId" TEXT NOT NULL,
    "role" INTEGER NOT NULL
);


INSERT INTO "User" VALUES ('clah4lqt00004m1fdre6wsza0', 'Demo', 'demo@company.com', '2022-11-14 18:35:27.348', '2022-11-14 18:35:27.348', '$2a$10$Jag2FanVbwvb00HbtOyWRufyZo4svPw4dF8u6V2nEtmFk55IXWbpC', NULL, '', NULL, NULL);