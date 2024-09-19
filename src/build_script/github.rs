//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use tokio::task::JoinSet;
use octocrab::instance;

use crate::*;

async fn get_license_text_from_github(url: &String) -> Option<String> {
    let split_url: Vec<&str> = url.split("/").collect();
    let length = split_url.len();

    if length < 3 {
        return None;        
    }

    let owner = split_url[length-2];
    let mut repo_str = split_url[length-1];
    if let Some(repo_str_stripped) = repo_str.strip_suffix(".git") {
        repo_str = repo_str_stripped;
    }
    let octo = instance();
    let repo = octo.repos(owner, repo_str);

    if let Ok(content) = repo.license().await {
        content.decoded_content()
    } else {
        None
    }
}

pub(super) async fn get_license_text_from_github_for_package_list(package_list: PackageList) -> PackageList {
    let mut set = JoinSet::new();

    let mut packages_with_license = PackageList(vec![]);

    for package in package_list.0.into_iter() {
        if !package.license_text.is_none() {
            packages_with_license.0.push(package);
            continue;
        }

        if let Some(repo_url) = &package.repository {
            if repo_url.contains("github") {
                set.spawn(async move {
                    let mut pack = package;
                    pack.license_text = get_license_text_from_github(pack.repository.as_ref().unwrap()).await;

                    

                    pack
                });
                continue;
            }
        }
        packages_with_license.0.push(package);
    }

    while let Some(pack_res) = set.join_next().await {
        if let Ok(pack) = pack_res {
            packages_with_license.0.push(pack)
        }
    }

    packages_with_license
}