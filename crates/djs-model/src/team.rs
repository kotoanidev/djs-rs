//! [Teams](https://docs.discord.com/developers/topics/teams): groups of developers that own
//! applications together.

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::{
    id::{TeamId, UserId},
    macros::{int_enum, str_enum},
    user::User,
    util::ImageHash,
};

/// A [team](https://docs.discord.com/developers/topics/teams#data-models-team-object).
///
/// discord.js counterpart: `Team`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Team {
    /// The team's [icon hash](https://docs.discord.com/developers/reference#image-formatting).
    pub icon: Option<ImageHash>,
    /// The team's ID.
    pub id: TeamId,
    /// The team's members.
    pub members: Vec<Member>,
    /// The team's name.
    pub name: CompactString,
    /// The current owner.
    pub owner_user_id: UserId,
}

/// A [member of a team](https://docs.discord.com/developers/topics/teams#data-models-team-member-object).
///
/// discord.js counterpart: `TeamMember`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[doc(alias = "TeamMember")]
pub struct Member {
    /// Whether the member accepted the invitation.
    pub membership_state: MembershipState,
    /// The team.
    pub team_id: TeamId,
    /// The member's user (only `avatar`, `discriminator`, `id` and `username` are set).
    pub user: User,
    /// The member's role. The owner is identified by [`Team::owner_user_id`] instead.
    pub role: MemberRole,
}

int_enum! {
    /// The [membership state](https://docs.discord.com/developers/topics/teams#data-models-membership-state-enum)
    /// of a team member.
    ///
    /// discord.js counterpart: `TeamMemberMembershipState`.
    pub enum MembershipState: u8 {
        /// Invited, not accepted yet.
        Invited = 1,
        /// Accepted the invitation.
        Accepted = 2,
    }
}

str_enum! {
    /// The [role of a team member](https://docs.discord.com/developers/topics/teams#team-member-roles-team-member-role-types).
    ///
    /// discord.js counterpart: `TeamMemberRole`.
    #[doc(alias = "TeamMemberRole")]
    pub enum MemberRole {
        /// Like the owner, but cannot take destructive actions on the team or its apps.
        Admin = "admin",
        /// Can see team apps' secrets and change their settings, but not the team.
        Developer = "developer",
        /// Can see the team and its apps.
        ReadOnly = "read_only",
    }
}
